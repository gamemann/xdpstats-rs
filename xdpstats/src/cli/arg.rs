use clap::Parser;

use crate::logger::{base::BACKLOG_DEFAULT_SZ, level::LogLevel};

#[derive(Debug, Parser, Clone)]
#[command(author, version, about, long_about = None)]
pub struct CliOpts {
    #[arg(
        short = 'L',
        long = "log",
        default_value_t = LogLevel::Info,
        help = "The log level to use."
    )]
    pub log_level: LogLevel,

    #[arg(
        short = 'l',
        long = "list",
        default_value_t = false,
        help = "Lists all available network interfaces and exits."
    )]
    pub list: bool,

    #[arg(
        short = 'B',
        long = "backlog",
        default_value_t = BACKLOG_DEFAULT_SZ,
        help = "Maximum amount of logs to store in stdout buffer when using the watch mode."
    )]
    pub backlog: usize,

    #[arg(
        short = 'i',
        long = "iface",
        default_value = "eth0",
        help = "The interface(s) to attach the XDP program to. Multiple interfaces can be specified by separating them with a comma (e.g. 'eth0,eth1')."
    )]
    pub iface: String,

    #[arg(
        short = 'd',
        long = "duration",
        default_value_t = 0,
        help = "The amount of time in seconds to run the program for (0 = infinite)."
    )]
    pub duration: u64,

    #[arg(
        short = 'a',
        long = "afxdp",
        default_value_t = false,
        help = "Forwards and processes packets in AF_XDP sockets instead of the raw XDP program."
    )]
    pub afxdp: bool,

    #[arg(
        short = 'n',
        long = "socks",
        default_value_t = 0,
        help = "The amount of AF_XDP sockets to create when running in AF_XDP mode."
    )]
    pub afxdp_num_socks: u32,

    #[arg(
        short = 'b',
        long = "batch-size",
        default_value_t = 64,
        help = "The batch size to use when processing packets in AF_XDP mode."
    )]
    pub afxdp_batch_size: u32,

    #[arg(
        short = 'r',
        long = "rx-sz",
        default_value_t = 2048,
        help = "The RX ring size."
    )]
    pub afxdp_rx_q_size: u32,

    #[arg(
        short = 't',
        long = "tx-sz",
        default_value_t = 2048,
        help = "The TX ring size."
    )]
    pub afxdp_tx_q_size: u32,

    #[arg(
        short = 'c',
        long = "cq-sz",
        default_value_t = 2048,
        help = "The completion queue size."
    )]
    pub afxdp_cq_size: u32,

    #[arg(
        short = 'f',
        long = "frame-sz",
        default_value_t = 2048,
        help = "The frame size to use when processing packets in AF_XDP mode."
    )]
    pub afxdp_frame_size: u32,

    #[arg(
        short = 'm',
        long = "frame-cnt",
        default_value_t = 4096,
        help = "The frame count to use when processing packets in AF_XDP mode."
    )]
    pub afxdp_frame_count: u32,

    #[arg(
        short = 'u',
        long = "wakeup",
        default_value_t = false,
        help = "If set, AF_XDP sockets will be configured to require wakeups when sending packets. This can reduce latency at the cost of increased CPU usage."
    )]
    pub afxdp_need_wakeup: bool,

    #[arg(
        short = 'x',
        long = "zero-copy",
        default_value_t = true,
        help = "If set, AF_XDP sockets will be configured to use zero-copy. This can reduce latency and CPU usage, but requires that the UMEM be pinned in memory and cannot be shared across multiple sockets."
    )]
    pub afxdp_zero_copy: bool,

    #[arg(
        short = 'S',
        long = "shared-umem",
        default_value_t = false,
        help = "If set, AF_XDP sockets will be configured to share a UMEM. This can reduce memory usage, but requires that zero-copy be disabled."
    )]
    pub afxdp_shared_umem: bool,

    #[arg(
        short = 'q',
        long = "queue-id",
        help = "The queue ID to bind AF_XDP sockets to. If not set, sockets will be bound to queue 0."
    )]
    pub afxdp_queue_id: Option<u16>,

    #[arg(
        short = 'P',
        long = "poll-timeout",
        default_value_t = 100,
        help = "The timeout in milliseconds to use when polling AF_XDP sockets."
    )]
    pub afxdp_poll_ms_timeout: i32,

    #[arg(
        short = 'F',
        long = "fq-sz",
        default_value_t = 2048,
        help = "The fill queue size to use when processing packets in AF_XDP mode."
    )]
    pub afxdp_fq_size: u32,

    #[arg(
        short = 'w',
        long = "watch",
        default_value_t = false,
        help = "If set, displays real-time counters with a graph."
    )]
    pub watch: bool,

    #[arg(
        short = 's',
        long = "skb",
        default_value_t = false,
        help = "If set, attaches the XDP program using SKB mode (slower) instead of DRV."
    )]
    pub skb: bool,

    #[arg(
        short = 'o',
        long = "offload",
        default_value_t = false,
        help = "If set, attaches the XDP program using offload mode instead of DRV."
    )]
    pub offload: bool,

    #[arg(
        short = 'R',
        long = "replace",
        default_value_t = false,
        help = "If set, will attempt to replace the XDP program if it is already attached to the interface(s)."
    )]
    pub replace: bool,

    #[arg(
        short = 'p',
        long = "per-sec",
        default_value_t = false,
        help = "Shows stats per second instead of total."
    )]
    pub per_sec: bool,

    #[arg(
        short = 'N',
        long = "sec-name",
        default_value = "xdp_stats",
        help = "The section name of the XDP program in the BPF object file."
    )]
    pub sec_name: String,
}

impl CliOpts {
    pub fn get_ifaces(&self) -> Vec<String> {
        self.iface
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }
}
