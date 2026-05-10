use anyhow::{Context as AnyhowCtx, Result};

use std::io::Stdout;

use ratatui::{Terminal, backend::CrosstermBackend};

use crate::context::Context;

pub struct Watcher {
    pub ctx: Context,

    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Watcher {
    pub fn new(ctx: Context) -> Result<Self> {
        let stdout = Self::get_stdout();

        // Retrieve the backend.
        let backend = CrosstermBackend::new(stdout);

        let terminal = Terminal::new(backend).context("Failed to initialize terminal")?;

        Ok(Self { ctx, terminal })
    }

    pub fn get_stdout() -> Stdout {
        std::io::stdout()
    }
}
