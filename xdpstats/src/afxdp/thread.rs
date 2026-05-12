use std::{
    os::fd::AsRawFd,
    sync::{Arc, Mutex},
};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use aya::maps::{MapData, PerCpuArray, XskMap};
use xdpstats_common::StatVal;

use crate::{
    afxdp::{
        packet::process_packet,
        socket::{XskTxConfig, XskTxSocket, XskUmem},
        stats::AfxdpStats,
    },
    context::Context,
    debug, info, warn,
};

pub fn thread_process(
    thread_id: u32,
    ctx: Context,
    shared_umem: Option<XskUmem>,
    iface: &str,
) -> Result<()> {
    debug!(
        ctx.logger.blocking_read(),
        "Starting AF_XDP thread with ID {} on interface {}", thread_id, iface
    );

    // Create AF_XDP socket config.
    let cfg = XskTxConfig {
        if_name: iface.to_string(),
        queue_id: ctx.opts.afxdp_queue_id.unwrap_or(thread_id as u16),
        rx_q_size: ctx.opts.afxdp_rx_q_size,
        tx_q_size: ctx.opts.afxdp_tx_q_size,
        cq_size: ctx.opts.afxdp_cq_size,
        fq_size: ctx.opts.afxdp_fq_size,
        frame_size: ctx.opts.afxdp_frame_size,
        frame_count: ctx.opts.afxdp_frame_count,
        batch_size: ctx.opts.afxdp_batch_size as usize,
        need_wakeup: ctx.opts.afxdp_need_wakeup,
        zero_copy: ctx.opts.afxdp_zero_copy,
        shared_umem: shared_umem.is_some(),
        poll_ms_timeout: ctx.opts.afxdp_poll_ms_timeout,
    };

    // Create the socket now.
    let mut sock = XskTxSocket::new(cfg, shared_umem.as_ref())
        .map_err(|e| anyhow!("Failed to create AF_XDP socket: {e}"))?;

    // We need to retrieve the XSK map and update it with our socket FD.
    {
        let mut prog_lock = ctx.xdp_prog.blocking_lock();

        let map_mut = match prog_lock.prog_bpf.map_mut("MAP_XSK") {
            Some(map) => map,
            None => {
                warn!(
                    ctx.logger.blocking_read(),
                    "Failed to retrieve XSK map from BPF program"
                );

                return Err(anyhow!("Failed to retrieve XSK map from BPF program"));
            }
        };

        let mut xsk_map = match XskMap::try_from(map_mut) {
            Ok(map) => map,
            Err(e) => {
                warn!(
                    ctx.logger.blocking_read(),
                    "Failed to convert map to XSK map: {e}"
                );

                return Err(anyhow!("Failed to convert map to XSK map: {e}"));
            }
        };

        // Retrieve the socket ID (available from the queues).
        let fd = sock.rx_q.fd().as_raw_fd();

        // We then store the FD in the XSK map at the thread ID index.
        // This is how the XDP program will know which socket to send packets to for this thread/queue.
        xsk_map
            .set(thread_id, fd, 0)
            .context("Failed to set socket FD in XSK map")?;

        info!(
            ctx.logger.blocking_read(),
            "Successfully set socket FD {} in XSK map at index {}", fd, thread_id
        );
    }

    // Now retrieve the stats map now to save some time in the packet processing loop.
    let mut stats = AfxdpStats::new(ctx.xdp_prog.blocking_lock().stats_map.clone(), thread_id);

    loop {
        if ctx.token.is_cancelled() {
            break;
        }

        if let Err(e) = sock.recv(
            ctx.opts.afxdp_poll_ms_timeout,
            ctx.opts.afxdp_need_wakeup,
            |frame_data| process_packet(frame_data),
            ctx.clone(),
            &mut stats,
        ) {
            warn!(ctx.logger.blocking_read(), "Failed to receive packets: {e}");
        }

        if let Err(e) = sock.complete_tx(ctx.opts.afxdp_need_wakeup) {
            warn!(ctx.logger.blocking_read(), "Failed to complete TX: {e}");
        }
    }

    Ok(())
}
