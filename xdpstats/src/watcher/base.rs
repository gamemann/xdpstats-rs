use anyhow::{Context as AnyhowCtx, Result};
use std::{
    collections::VecDeque,
    io::Stdout,
    sync::{Arc, Mutex},
};

use crate::context::Context;
use ratatui::{Terminal, backend::CrosstermBackend};

pub const HISTORY_LEN: usize = 60;

pub type LogBuffer = Arc<Mutex<VecDeque<String>>>;

pub enum ViewMode {
    Packets,
    Bytes,
}

pub struct Watcher {
    pub ctx: Context,
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    pub logs: Option<LogBuffer>,
    pub view_mode: ViewMode,
    pub history_pkt: [VecDeque<f64>; 5],
    pub history_byt: [VecDeque<f64>; 5],
}

impl Watcher {
    pub fn new(ctx: Context, logs: Option<LogBuffer>) -> Result<Self> {
        let stdout = Self::get_stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to initialize terminal")?;

        Ok(Self {
            ctx,
            terminal,
            logs,
            view_mode: ViewMode::Packets,
            history_pkt: Default::default(),
            history_byt: Default::default(),
        })
    }

    pub fn get_stdout() -> Stdout {
        std::io::stdout()
    }
}
