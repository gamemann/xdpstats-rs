mod afxdp;
mod cli;
mod logger;
mod util;
mod watcher;
mod xdp;

mod context;

use cli::CliOpts;
use util::raise_rlimit;

use anyhow::{Result, anyhow};
use aya::programs::XdpFlags;
use std::collections::VecDeque;
#[rustfmt::skip]
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::{select, signal};

use tokio::time::{self, Instant, interval};

use clap::Parser;

use crate::context::ContextData;
use crate::logger::base::Logger;
use crate::watcher::base::{LogBuffer, Watcher};
use crate::xdp::base::Xdp;

#[tokio::main]
async fn main() -> Result<()> {
    // Let's parse our CLI options first and extract the input.
    let opts = match CliOpts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            return Err(anyhow!("Failed to parse CLI options: {e}"));
        }
    };

    if opts.list {
        opts.list();

        return Ok(());
    }

    // Create log buffer for watcher if necessary.
    let logs_buff: Option<LogBuffer> = {
        if opts.watch {
            Some(Arc::new(Mutex::new(VecDeque::with_capacity(opts.backlog))))
        } else {
            None
        }
    };

    // Setup our logger.
    let logger = Logger::new(opts.log_level, logs_buff.clone(), opts.backlog);

    // We need to retrieve the list of interfaces to attach to.
    let mut ifaces = opts.get_ifaces();

    // If we don't have any interfaces, we can try eth0, but warn.
    if ifaces.is_empty() {
        warn!(
            logger,
            "No interfaces specified, attempting to use 'eth0'..."
        );

        ifaces.push("eth0".to_string());
    }

    // We need to raise the RLimit for older kernels.
    match raise_rlimit() {
        Err(e) => warn!(logger, "Failed to raise rlimit: {e}"),
        Ok(_) => debug!(logger, "Successfully raised rlimit"),
    };

    // Initialize our XDP structure.
    // When initializing via new, it loads the BPF program.
    let mut xdp_prog = match Xdp::new(&opts.sec_name) {
        Ok(xdp) => xdp,
        Err(e) => {
            fatal!(logger, "Failed to initialize XDP program: {e}");

            return Err(e);
        }
    };

    // Attemp to load the XDP program.
    match xdp_prog.load() {
        Ok(_) => debug!(logger, "Successfully loaded XDP program"),
        Err(e) => {
            error!(logger, "Failed to load XDP program: {e}");

            return Err(e);
        }
    }

    // Before attaching the XDP program, let's compile the attach flags from input.
    let mut attach_flags = match (opts.skb, opts.offload) {
        (true, false) => XdpFlags::SKB_MODE,
        (false, true) => XdpFlags::HW_MODE,
        _ => XdpFlags::default(), // XdpFlags::default() attaches using DRV mode from my testing.
                                  // If it won't load with DRV mode, you could try XdpFlags::DRV_MODE directly.
    };

    // Apply the replace flag if wanted.
    // This would be ideal by default, but
    // For some reason this causes a panic with:
    // called `Option::unwrap()` on a `None` value
    // When attaching below
    if opts.replace {
        attach_flags |= XdpFlags::REPLACE;
    }

    // Now attempt to load XDP programs on interfaces specified.
    let mut is_attached = false;

    for iface in ifaces {
        // Try checking if the interface is valid.

        match xdp_prog.attach(iface.as_str(), attach_flags) {
            Ok(_) => {
                if !is_attached {
                    is_attached = true;
                }

                info!(logger, "Attached XDP program to interface {iface}...");
            }
            Err(e) => warn!(
                logger,
                "Failed to attach XDP program to interface '{iface}': {e}"
            ),
        }
    }

    // If we aren't attached, exit.
    if !is_attached {
        return Err(anyhow::anyhow!(
            "Failed to attach XDP program to any interface"
        ));
    }

    // Attempt to insert first stats entry value.
    match xdp_prog.stats_init() {
        Ok(_) => debug!(
            logger,
            "Successfully initialized stats and inserted first value into map"
        ),
        Err(e) => warn!(logger, "Failed to insert first stats entry value: {e}"),
    }

    // Create context and move what we need into it.
    let ctx = ContextData::new(opts, xdp_prog, logger);

    // Spawn AF_XDP sockets if we're using the feature.
    #[cfg(feature = "afxdp")]
    {
        use crate::afxdp::init::setup_sockets;

        match setup_sockets(ctx.clone()).await {
            Ok(_) => info!(
                ctx.logger.read().await,
                "Successfully set up AF_XDP sockets"
            ),
            Err(e) => warn!(
                ctx.logger.read().await,
                "Failed to set up AF_XDP sockets: {e}"
            ),
        }
    }

    // Spawn a task to calculate stats every second and check for duration.
    let ctx_clone = ctx.clone();

    tokio::spawn(async move {
        let mut check_interval = interval(Duration::from_millis(300));

        let start_time = Instant::now();

        loop {
            select! {
                _ = check_interval.tick() => {
                    if ctx_clone.opts.duration > 0 && start_time.elapsed().as_secs() >= ctx_clone.opts.duration {
                        ctx_clone.token.cancel();

                        break;
                    }


                    let mut xdp_prog = ctx_clone.xdp_prog.lock().await;

                    match xdp_prog.stats_calc(ctx_clone.opts.per_sec) {
                        Ok(_) => debug!(ctx_clone.logger.read().await, "Successfully calculated stats from map"),
                        Err(e) => warn!(ctx_clone.logger.read().await, "Failed to calculate stats: {e}"),
                    }
                }
            }
        }
    });

    info!(
        ctx.logger.read().await,
        "Rust XDP Stats loaded! Please use CTRL + C to exit..."
    );

    if ctx.opts.watch {
        // Create wather
        let mut watcher = match Watcher::new(ctx.clone(), logs_buff.clone()) {
            Ok(w) => w,
            Err(e) => {
                error!(ctx.logger.read().await, "Failed to initialize watcher: {e}");

                return Err(e);
            }
        };

        match watcher.run().await {
            Ok(_) => {}
            Err(e) => {
                error!(ctx.logger.read().await, "Watcher encountered an error: {e}");

                return Err(e);
            }
        }
    } else {
        // We need to calculate our interval (one second).
        let mut interval = time::interval(Duration::from_millis(1000));

        loop {
            select! {
                _ = interval.tick() => {
                    {
                        match ctx.xdp_prog.lock().await.stats_display_pretty(ctx.opts.per_sec, true) {
                            Ok(_) => {},
                            Err(e) => warn!(ctx.logger.read().await, "Failed to display stats: {e}"),
                        }
                    }
                },
                _ = signal::ctrl_c() => {
                    info!(ctx.logger.read().await, "Found CTRL + C signal, exiting...");

                    ctx.token.cancel();

                    break;
                }
                _ = ctx.token.cancelled() => {
                    info!(ctx.logger.read().await, "Duration elapsed, exiting...");

                    break;
                }
            }
        }
    }

    println!();

    info!(
        ctx.logger.read().await,
        "Rust XDP Stats cleaned up and exiting..."
    );

    Ok(())
}
