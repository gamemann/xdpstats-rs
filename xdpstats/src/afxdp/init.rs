use std::thread;

use anyhow::{Result, anyhow};
use log::warn;

use crate::{
    afxdp::{
        opt::AfXdpOpts,
        socket::{XskTxConfig, XskUmem},
        thread::thread_process,
    },
    context::Context,
};

pub fn setup_sockets(ctx: Context) -> Result<()> {
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

    for t_id in 0..ctx.opts.afxdp_num_socks {
        let ctx = ctx.clone();
        let umem = umem.clone();

        let iface = iface.clone();

        // Create thread and execute our processing function inside of it.
        let t = thread::spawn(
            move || match thread_process(t_id, ctx.clone(), umem, &iface) {
                Ok(_) => (),
                Err(e) => warn!("Thread {} encountered an error: {}", t_id, e),
            },
        );

        threads.push(t);
    }

    Ok(())
}
