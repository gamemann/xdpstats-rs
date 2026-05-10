use std::sync::{Arc, atomic::AtomicBool};

use tokio::sync::{Mutex, RwLock};

use crate::{cli::CliOpts, logger::base::Logger, xdp::base::Xdp};

pub struct ContextData {
    pub running: Arc<AtomicBool>,

    pub opts: CliOpts,

    pub xdp_prog: Mutex<Xdp>,
    pub logger: RwLock<Logger>,
}

pub type Context = Arc<ContextData>;

impl ContextData {
    pub fn new(opts: CliOpts, xdp_prog: Xdp, running: Arc<AtomicBool>, logger: Logger) -> Context {
        Arc::new(Self {
            running,
            opts,
            xdp_prog: Mutex::new(xdp_prog),
            logger: RwLock::new(logger),
        })
    }
}
