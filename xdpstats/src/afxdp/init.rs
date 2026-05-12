use std::thread;

use crate::{
    afxdp::{
        opt::AfXdpOpts,
        socket::{XskTxConfig, XskUmem},
        thread::thread_process,
    },
    context::Context,
    debug, warn,
};
use anyhow::{Result, anyhow};

pub async fn setup_sockets(ctx: Context) -> Result<()> {
    debug!(ctx.logger.read().await, "Setting up AF_XDP sockets...");

    let cfg = XskTxConfig::from(AfXdpOpts::from(ctx.opts.clone()));

    // Create a shared UMEM if requested.
    let umem = if cfg.shared_umem {
        Some(XskUmem::new(&cfg)?)
    } else {
        None
    };

    // Create the socket(s).
    let mut threads = Vec::new();

    let iface = ctx
        .opts
        .get_ifaces()
        .get(0)
        .ok_or_else(|| anyhow!("no interfaces specified"))
        .map(|iface| iface.clone())?;

    let socks = if ctx.opts.afxdp_num_socks == 0 {
        num_cpus::get() as u32
    } else {
        ctx.opts.afxdp_num_socks as u32
    };

    for t_id in 0..socks {
        let ctx = ctx.clone();

        let umem = umem.clone();

        let iface = iface.clone();

        // Create thread and execute our processing function inside of it.
        let t = thread::spawn(
            move || match thread_process(t_id, ctx.clone(), umem, &iface) {
                Ok(_) => (),
                Err(e) => warn!(
                    ctx.logger.blocking_read(),
                    "Thread {} encountered an error: {}", t_id, e
                ),
            },
        );

        threads.push(t);
    }

    Ok(())
}
