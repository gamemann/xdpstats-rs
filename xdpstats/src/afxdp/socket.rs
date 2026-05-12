use anyhow::{Context as AnyhowCtx, Result, anyhow};
use std::{io::Write, num::NonZeroU32};
use xdpstats_common::StatType;
use xsk_rs::{
    CompQueue, FillQueue, FrameDesc, RxQueue, Socket, TxQueue, Umem,
    config::{
        BindFlags, FrameSize, Interface, LibxdpFlags, QueueSize, SocketConfig, UmemConfig, XdpFlags,
    },
};

use crate::{
    afxdp::{opt::AfXdpOpts, stats::AfxdpStats},
    context::Context,
    warn,
};

pub enum Action {
    Drop,
    Tx,
}

pub struct XskTxSocket {
    pub umem: Umem,

    pub fq: FillQueue,
    pub cq: CompQueue,

    pub rx_q: RxQueue,
    pub tx_q: TxQueue,
    rx_free: Vec<FrameDesc>,
    tx_free: Vec<FrameDesc>,

    pub batch_size: usize,
    pub outstanding_tx: u32,

    cq_scratch: Vec<FrameDesc>,
    rx_scratch: Vec<FrameDesc>,
}

pub struct XskTxConfig {
    pub if_name: String,
    pub queue_id: u16,
    pub rx_q_size: u32,
    pub tx_q_size: u32,
    pub cq_size: u32,
    pub fq_size: u32,
    pub frame_size: u32,
    pub frame_count: u32,
    pub batch_size: usize,
    pub need_wakeup: bool,
    pub zero_copy: bool,
    pub shared_umem: bool,
    pub poll_ms_timeout: i32,
}

impl From<AfXdpOpts> for XskTxConfig {
    fn from(opts: AfXdpOpts) -> Self {
        Self {
            if_name: String::new(),
            queue_id: opts.queue_id.unwrap_or(0),
            rx_q_size: opts.rx_q_size,
            tx_q_size: opts.tx_q_size,
            cq_size: opts.cq_size,
            fq_size: opts.fq_size,
            frame_size: opts.frame_size,
            frame_count: opts.frame_count,
            batch_size: opts.batch_size as usize,
            need_wakeup: opts.need_wakeup,
            zero_copy: opts.zero_copy,
            shared_umem: opts.shared_umem,
            poll_ms_timeout: opts.poll_ms_timeout,
        }
    }
}

/// Holds a UMEM that can optionally be shared across multiple sockets.
#[derive(Clone)]
pub struct XskUmem {
    pub umem: Umem,
    pub descs: Vec<FrameDesc>,
}

impl XskUmem {
    /// Create a new UMEM based on the provided configuration.
    ///
    /// # Arguments
    /// * `cfg` - The configuration to use when creating the UMEM.
    ///
    /// # Returns
    /// A result containing the created UMEM or an error if the UMEM could not be created.
    pub fn new(cfg: &XskTxConfig) -> Result<Self> {
        let frame_size: FrameSize = cfg.frame_size.try_into().context("invalid frame size")?;
        let cq_size: QueueSize = cfg.cq_size.try_into().context("invalid cq size")?;
        let fq_size: QueueSize = cfg.fq_size.try_into().context("invalid fq size")?;

        let umem_config = UmemConfig::builder()
            .frame_size(frame_size)
            .comp_queue_size(cq_size)
            .fill_queue_size(fq_size)
            .build()
            .map_err(|e| anyhow!("failed to build umem config: {}", e))?;

        let frame_count =
            NonZeroU32::new(cfg.frame_count).context("frame count must be non-zero")?;

        let (umem, descs) = Umem::new(umem_config, frame_count, cfg.zero_copy)
            .map_err(|e| anyhow!("failed to create UMEM: {}", e))?;

        Ok(Self { umem, descs })
    }
}

impl XskTxSocket {
    /// Create a socket with its own dedicated UMEM.
    pub fn new(cfg: XskTxConfig, shared_umem: Option<&XskUmem>) -> Result<Self> {
        let owned_umem;

        let umem = match shared_umem {
            Some(shared) => shared,
            None => {
                owned_umem = XskUmem::new(&cfg)
                    .map_err(|e| anyhow!("failed to create UMEM for socket: {}", e))?;
                &owned_umem
            }
        };

        // We need to split the UMEM frames into RX and TX sets.
        let total = umem.descs.len();

        let tx_count = (total / 2).max(cfg.batch_size * 2).min(total);

        let tx_free: Vec<FrameDesc> = umem.descs[..tx_count].to_vec();
        let rx_descs: Vec<FrameDesc> = umem.descs[tx_count..].to_vec();

        // Build socket flags.
        let bind_flags = Self::build_bind_flags(&cfg);
        let libxdp_flags = Self::build_libxdp_flags();
        let xdp_flags = Self::build_xdp_flags(&cfg);

        // Build socket config.
        let sock_cfg = SocketConfig::builder()
            .rx_queue_size(cfg.rx_q_size.try_into().context("invalid rx queue size")?)
            .tx_queue_size(cfg.tx_q_size.try_into().context("invalid tx queue size")?)
            .bind_flags(bind_flags)
            .libxdp_flags(libxdp_flags)
            .xdp_flags(xdp_flags)
            .build();

        // Parse interface name and ensure it's valid.
        let if_name: Interface = cfg.if_name.parse().context("invalid interface name")?;

        let (tx_q, rx_q, fq_and_cq) = unsafe {
            Socket::new(sock_cfg, &umem.umem, &if_name, cfg.queue_id as u32)
                .context("failed to create AF_XDP socket")?
        };

        let (mut fq, cq) =
            fq_and_cq.context("failed to get fill/comp queues for shared umem socket")?;

        // Stuff the fill queue with all RX descriptors.
        // This ensures the kernel can place packets into the RX ring immediately.
        let n = unsafe { fq.produce(&rx_descs[..]) };

        if n == 0 {
            return Err(anyhow!("fill queue rejected all initial RX descriptors"));
        }

        Ok(Self {
            umem: umem.umem.clone(),
            fq,
            cq,
            rx_q,
            tx_q,
            rx_free: Vec::new(),
            tx_free,
            batch_size: cfg.batch_size,
            outstanding_tx: 0,

            cq_scratch: vec![FrameDesc::default(); cfg.batch_size],
            rx_scratch: vec![FrameDesc::default(); cfg.batch_size],
        })
    }

    fn build_bind_flags(cfg: &XskTxConfig) -> BindFlags {
        let mut flags = BindFlags::empty();

        if cfg.zero_copy {
            flags |= BindFlags::XDP_ZEROCOPY;
        } else {
            flags |= BindFlags::XDP_COPY;
        }

        flags
    }

    fn build_libxdp_flags() -> LibxdpFlags {
        LibxdpFlags::XSK_LIBXDP_FLAGS_INHIBIT_PROG_LOAD
    }

    fn build_xdp_flags(cfg: &XskTxConfig) -> XdpFlags {
        let mut flags = XdpFlags::empty();

        if cfg.zero_copy {
            flags |= XdpFlags::XDP_FLAGS_DRV_MODE;
        } else {
            flags |= XdpFlags::XDP_FLAGS_SKB_MODE;
        }

        flags
    }

    #[inline(always)]
    pub fn recv<F>(
        &mut self,
        poll_ms_timeout: i32,
        check_for_wakeup: bool,
        mut handler: F,
        ctx: Context,
        stats: &mut AfxdpStats,
    ) -> Result<usize>
    where
        F: FnMut(&[u8]) -> Action,
    {
        let n = unsafe {
            self.rx_q
                .poll_and_consume(&mut self.rx_scratch[..], poll_ms_timeout)
                .context("rx poll_and_consume failed")?
        };

        if n == 0 {
            // Check if we should wakeup.
            if check_for_wakeup {
                self.maybe_wakeup_fq(poll_ms_timeout);
            }

            return Ok(0);
        }

        // Grave frames from the RX scratch.
        let frames: Vec<FrameDesc> = self.rx_scratch[..n].iter().copied().collect();

        for desc in frames {
            let pkt_bytes: &[u8] = unsafe { self.umem.data(&desc).contents() };

            // Retrieve the action.
            // Dedicated handler function isn't needed, but it makes the code cleaner and this should set an example.
            let pkt_len = pkt_bytes.len() as u64;
            let action = handler(pkt_bytes);

            // Retrieve stats type based on the action and whether we are able to enqueue for TX if needed.
            let stat_type = match action {
                Action::Tx => {
                    match self.enqueue_tx(desc) {
                        Ok(_) => StatType::MATCH,
                        Err(e) => {
                            // If we fail to enqueue for TX, we should still recycle the frame back to RX.
                            self.rx_free.push(desc);

                            warn!(
                                ctx.logger.blocking_read(),
                                "Failed to enqueue packet for TX: {}", e
                            );

                            StatType::DROP
                        }
                    }
                }
                Action::Drop => {
                    self.rx_free.push(desc);

                    StatType::MATCH
                }
            };

            // Increment stats.
            match stats.inc(stat_type, pkt_len as u64) {
                Ok(_) => {}
                Err(e) => {
                    warn!(ctx.logger.blocking_read(), "Failed to increment stats: {e}");
                }
            }
        }

        self.refill_fq(poll_ms_timeout);

        Ok(n)
    }

    #[inline(always)]
    pub fn send(&mut self, pkt: &[u8]) -> Result<()> {
        // Drain the completion queue first if we're out of TX frames.
        if self.tx_free.is_empty() {
            self.drain_cq();
        }

        let mut desc = self
            .tx_free
            .pop()
            .ok_or_else(|| anyhow!("TX frame pool exhausted"))?;

        unsafe {
            self.umem
                .data_mut(&mut desc)
                .cursor()
                .write_all(pkt)
                .context("write to UMEM frame failed")?;
        }

        self.enqueue_tx(desc)
    }

    /// Best-effort flush: wake the NIC driver and drain all outstanding completions.
    pub fn complete_tx(&mut self, need_wakeup: bool) -> Result<()> {
        if need_wakeup {
            self.tx_q.wakeup().ok();
        }

        while self.outstanding_tx > 0 {
            self.drain_cq();
            if self.outstanding_tx > 0 {
                std::hint::spin_loop();
            }
        }
        Ok(())
    }

    fn enqueue_tx(&mut self, desc: FrameDesc) -> Result<()> {
        loop {
            match unsafe { self.tx_q.produce_and_wakeup(&[desc]) } {
                Ok(n) if n > 0 => break,
                _ => {
                    // TX ring full — drain completions and retry.
                    self.drain_cq();
                }
            }
        }

        self.outstanding_tx += 1;

        if self.outstanding_tx >= self.batch_size as u32 {
            self.drain_cq();
        }

        Ok(())
    }

    fn drain_cq(&mut self) {
        let mut scratch: Vec<FrameDesc> = vec![FrameDesc::default(); self.batch_size];
        let n = unsafe { self.cq.consume(&mut scratch[..]) };
        for d in scratch[..n].iter().copied() {
            self.tx_free.push(d);
        }
        self.outstanding_tx = self.outstanding_tx.saturating_sub(n as u32);
    }

    fn refill_fq(&mut self, poll_ms_timeout: i32) {
        if self.rx_free.is_empty() {
            return;
        }

        // Try to push everything back; loop until the ring accepts them all.
        let mut remaining = self.rx_free.len();
        while remaining > 0 {
            let n = unsafe {
                let fd = self.rx_q.fd_mut();
                self.fq
                    .produce_and_wakeup(&self.rx_free[..remaining], fd, poll_ms_timeout)
                    .unwrap_or(0)
            };
            remaining -= n;
            if n == 0 {
                break; // ring is full, we'll retry next recv() call
            }
        }

        let leftover = remaining;
        self.rx_free.drain(..self.rx_free.len() - leftover);
    }

    fn maybe_wakeup_fq(&mut self, poll_ms_timeout: i32) {
        if self.fq.needs_wakeup() {
            let fd = self.rx_q.fd_mut();
            self.fq.wakeup(fd, poll_ms_timeout).ok();
        }
    }
}
