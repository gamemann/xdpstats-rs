use anyhow::{Context as AnyhowCtx, Result};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use crate::watcher::base::Watcher;

impl Watcher {
    /// Starts and runs the Watcher interface.
    ///
    /// # Returns
    /// A `Result` indicating success or failure. If the function encounters an error while setting up the terminal or reading stats, it will return an `anyhow::Error`.
    pub async fn run(&mut self) -> Result<()> {
        // We'll want to enable raw amode.
        enable_raw_mode()?;

        // Grab stdout and enter alternate screen for drawing.
        let mut stdout = Watcher::get_stdout();

        execute!(stdout, EnterAlternateScreen)?;

        // Hide the cursor while in the watcher.
        execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;

        // Clear the cursor so there isn't any overlapping text from before (cleanup)
        self.terminal.clear()?;

        // Disable raw mode now since we're done with the watcher.
        disable_raw_mode()?;

        self.interface_start()
            .await
            .context("Failed to start interface watcher")?;

        // Restore cursor and leave alternate screen
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;

        Ok(())
    }
}
