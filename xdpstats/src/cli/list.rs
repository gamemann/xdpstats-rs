use crate::cli::CliOpts;

impl CliOpts {
    pub fn list(&self) {
        let ifaces = self.get_ifaces();

        println!("Interfaces:");

        for iface in ifaces {
            println!(" - {iface}");
        }

        println!();

        println!("General Options:");
        println!("  Log Level => {}", self.log_level);
        println!("  Backlog Size => {}", self.backlog);
        println!("  Duration: {} seconds", self.duration);
        println!("  Watch: {}", self.watch);
        println!("  SKB Mode: {}", self.skb);
        println!("  Offload Mode: {}", self.offload);
        println!("  Replace: {}", self.replace);
        println!("  Per Second: {}", self.per_sec);
        println!("  Section Name: {}", self.sec_name);

        println!();

        println!("AF_XDP Options:");
        println!("  Enabled: {}", self.afxdp);
        println!("  Batch Size: {}", self.afxdp_batch_size);
        println!("  Need Wakeup: {}", self.afxdp_need_wakeup);
        println!("  Zero Copy: {}", self.afxdp_zero_copy);
        println!("  Shared UMEM: {}", self.afxdp_shared_umem);
        println!("  Queue ID: {}", self.afxdp_queue_id.unwrap_or(0));
        println!("  RX Queue Size: {}", self.afxdp_rx_q_size);
        println!("  TX Queue Size: {}", self.afxdp_tx_q_size);
        println!("  Completion Queue Size: {}", self.afxdp_cq_size);
        println!("  Fill Queue Size: {}", self.afxdp_fq_size);
        println!("  Frame Size: {}", self.afxdp_frame_size);
        println!("  Frame Count: {}", self.afxdp_frame_count);
        println!("  Poll Timeout: {}", self.afxdp_poll_ms_timeout);
    }
}
