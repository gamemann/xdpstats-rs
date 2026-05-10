use anyhow::Result;

use crate::watcher::base::Watcher;

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};
use tokio::time::sleep;

impl Watcher {
    pub async fn interface_start(&mut self) -> Result<()> {
        loop {
            if !self.ctx.running.load(Ordering::Relaxed) {
                break;
            }

            // Check for Ctrl+C keypress inside raw mode
            // We need this even though we have the tokio::select! signal in the main thread.
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.ctx.running.store(false, Ordering::Relaxed);

                        break;
                    }
                }
            }

            self.terminal.draw(|f| {
                let area = f.area();

                // Create chunks for current stats header, PPS chart, and BPS chart. The header will be a fixed height and the charts will split the remaining space.
                // Admittedly I used AI for this LOL, but I'm trying to learn Ratatui as well!
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),      // header with current stats
                        Constraint::Percentage(50), // PPS chart
                        Constraint::Percentage(50), // BPS chart
                    ])
                    .split(area);
            })?;

            // Sleep 1 second for next update (per second updates).
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}
