use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::{cli::CliOpts, logger::base::Logger, xdp::base::Xdp};

pub struct ContextData {
    pub opts: CliOpts,

    pub xdp_prog: Mutex<Xdp>,
    pub logger: RwLock<Logger>,

    pub token: CancellationToken,
}

pub type Context = Arc<ContextData>;

impl ContextData {
    pub fn new(opts: CliOpts, xdp_prog: Xdp, logger: Logger) -> Context {
        Arc::new(Self {
            opts,
            xdp_prog: Mutex::new(xdp_prog),
            logger: RwLock::new(logger),
            token: CancellationToken::new(),
        })
    }
}
