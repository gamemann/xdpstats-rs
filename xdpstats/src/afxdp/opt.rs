use crate::cli::arg::CliOpts;

#[derive(Debug, Clone)]
pub struct AfXdpOpts {
    pub iface: String,

    pub queue_id: Option<u16>,
    pub batch_size: u32,
    pub need_wakeup: bool,
    pub zero_copy: bool,
    pub shared_umem: bool,

    pub poll_ms_timeout: i32,

    pub rx_q_size: u32,
    pub tx_q_size: u32,

    pub cq_size: u32,
    pub fq_size: u32,

    pub frame_size: u32,
    pub frame_count: u32,
}

impl Default for AfXdpOpts {
    fn default() -> Self {
        Self {
            iface: String::new(),
            queue_id: None,
            batch_size: 64,
            need_wakeup: false,
            zero_copy: true,
            shared_umem: false,
            poll_ms_timeout: 100,
            rx_q_size: 2048,
            tx_q_size: 2048,
            cq_size: 2048,
            fq_size: 2048,
            frame_size: 2048,
            frame_count: 4096,
        }
    }
}

impl From<CliOpts> for AfXdpOpts {
    fn from(opts: CliOpts) -> Self {
        Self {
            iface: opts.iface.split(',').next().unwrap_or("").to_string(),
            batch_size: opts.afxdp_batch_size,
            need_wakeup: opts.afxdp_need_wakeup,
            zero_copy: opts.afxdp_zero_copy,
            shared_umem: opts.afxdp_shared_umem,
            poll_ms_timeout: opts.afxdp_poll_ms_timeout,
            queue_id: opts.afxdp_queue_id,
            rx_q_size: opts.afxdp_rx_q_size,
            tx_q_size: opts.afxdp_tx_q_size,
            cq_size: opts.afxdp_cq_size,
            fq_size: opts.afxdp_fq_size,
            frame_size: opts.afxdp_frame_size,
            frame_count: opts.afxdp_frame_count,
        }
    }
}
