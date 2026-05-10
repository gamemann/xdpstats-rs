use anyhow::Result;
use std::{collections::VecDeque, sync::atomic::Ordering, time::Duration};
use xdpstats_common::StatType;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};
use tokio::time::sleep;

use crate::{
    util::{format_byt, format_pkt},
    watcher::base::{HISTORY_LEN, ViewMode, Watcher},
};

struct StatMeta {
    label: &'static str,
    color: Color,
    stat_type: StatType,
}

const STATS: [StatMeta; 5] = [
    StatMeta {
        label: "Matched",
        color: Color::Blue,
        stat_type: StatType::MATCH,
    },
    StatMeta {
        label: "Error",
        color: Color::Red,
        stat_type: StatType::ERROR,
    },
    StatMeta {
        label: "Bad",
        color: Color::Yellow,
        stat_type: StatType::BAD,
    },
    StatMeta {
        label: "Dropped",
        color: Color::Magenta,
        stat_type: StatType::DROP,
    },
    StatMeta {
        label: "Passed",
        color: Color::Green,
        stat_type: StatType::PASS,
    },
];

impl Watcher {
    pub async fn interface_start(&mut self) -> Result<()> {
        loop {
            if !self.ctx.running.load(Ordering::Relaxed) {
                break;
            }

            // Handle keypresses.
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.ctx.running.store(false, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('1') => self.view_mode = ViewMode::Packets,
                        KeyCode::Char('2') => self.view_mode = ViewMode::Bytes,
                        _ => {}
                    }
                }
            }

            // Push current stats into history ring buffers.
            for (i, meta) in STATS.iter().enumerate() {
                if let Some(entry) = self
                    .ctx
                    .xdp_prog
                    .lock()
                    .await
                    .stats
                    .entry
                    .get(&meta.stat_type)
                {
                    push_history(&mut self.history_pkt[i], entry.cur.pkt as f64);
                    push_history(&mut self.history_byt[i], entry.cur.byt as f64);
                }
            }

            // Snapshot data for the draw closure.
            let history_pkt = self.history_pkt.clone();
            let history_byt = self.history_byt.clone();

            let logs_snapshot: Vec<String> = match &self.logs {
                Some(buffer) => buffer.lock().unwrap().iter().cloned().collect(),
                None => Vec::new(),
            };

            let stats_snapshot: Vec<Option<(u64, u64)>> = {
                let xdp_prog = self.ctx.xdp_prog.lock().await;

                for (i, meta) in STATS.iter().enumerate() {
                    if let Some(entry) = xdp_prog.stats.entry.get(&meta.stat_type) {
                        push_history(&mut self.history_pkt[i], entry.cur.pkt as f64);
                        push_history(&mut self.history_byt[i], entry.cur.byt as f64);
                    }
                }

                STATS
                    .iter()
                    .map(|m| {
                        xdp_prog
                            .stats
                            .entry
                            .get(&m.stat_type)
                            .map(|e| (e.cur.pkt, e.cur.byt))
                    })
                    .collect()
            };

            let is_packets = matches!(self.view_mode, ViewMode::Packets);

            self.terminal.draw(|f| {
                let area = f.area();

                // Outer layout: header | charts | logs
                let outer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(8),
                    ])
                    .split(area);

                // ── Header ────────────────────────────────────────────────
                let header_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(16), // Matched
                        Constraint::Percentage(16), // Error
                        Constraint::Percentage(16), // Bad
                        Constraint::Percentage(16), // Dropped
                        Constraint::Percentage(16), // Passed
                        Constraint::Percentage(20), // mode indicator
                    ])
                    .split(outer[0]);

                for (i, meta) in STATS.iter().enumerate() {
                    let (pkt, byt) = stats_snapshot[i].unwrap_or((0, 0));
                    let value = if is_packets {
                        format_pkt(pkt as f64, true)
                    } else {
                        format_byt(byt as f64, true)
                    };
                    let para = Paragraph::new(format!("{}: {}", meta.label, value))
                        .block(Block::default().borders(Borders::ALL))
                        .style(Style::default().fg(meta.color).add_modifier(Modifier::BOLD));
                    f.render_widget(para, header_chunks[i]);
                }

                // Mode indicator
                let (mode_text, mode_color) = if is_packets {
                    ("[1] Pkts  [2] Bytes", Color::Cyan)
                } else {
                    ("[1] Pkts  [2] Bytes", Color::Yellow)
                };
                let mode_para = Paragraph::new(mode_text)
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(mode_color).add_modifier(Modifier::BOLD));
                f.render_widget(mode_para, header_chunks[5]);

                // ── Charts ─────────────────────────────────────────────────
                let chart_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(20); 5])
                    .split(outer[1]);

                let active_history = if is_packets {
                    &history_pkt
                } else {
                    &history_byt
                };

                for (i, meta) in STATS.iter().enumerate() {
                    let data = to_chart_data(&active_history[i]);
                    let y_max = active_history[i].iter().cloned().fold(1.0_f64, f64::max) * 1.1;

                    let datasets = vec![
                        Dataset::default()
                            .marker(symbols::Marker::Braille)
                            .graph_type(GraphType::Line)
                            .style(Style::default().fg(meta.color))
                            .data(&data),
                    ];

                    let chart = Chart::new(datasets)
                        .block(
                            Block::default()
                                .title(Span::styled(
                                    meta.label,
                                    Style::default().fg(meta.color).add_modifier(Modifier::BOLD),
                                ))
                                .borders(Borders::ALL),
                        )
                        .x_axis(
                            Axis::default()
                                .bounds([0.0, HISTORY_LEN as f64])
                                .style(Style::default().fg(Color::DarkGray)),
                        )
                        .y_axis(
                            Axis::default()
                                .bounds([0.0, y_max])
                                .style(Style::default().fg(Color::DarkGray)),
                        );

                    f.render_widget(chart, chart_chunks[i]);
                }

                // ── Log pane ───────────────────────────────────────────────
                let log_text: Vec<Line> = logs_snapshot
                    .iter()
                    .rev()
                    .take(6)
                    .map(|l| Line::from(l.as_str()))
                    .collect();

                let log_para = Paragraph::new(log_text)
                    .block(
                        Block::default()
                            .title("Logs")
                            .borders(Borders::ALL)
                            .style(Style::default().fg(Color::DarkGray)),
                    )
                    .style(Style::default().fg(Color::Gray));

                f.render_widget(log_para, outer[2]);
            })?;

            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}

fn push_history(dq: &mut VecDeque<f64>, val: f64) {
    if dq.len() >= HISTORY_LEN {
        dq.pop_front();
    }
    dq.push_back(val);
}

fn to_chart_data(dq: &VecDeque<f64>) -> Vec<(f64, f64)> {
    let offset = HISTORY_LEN.saturating_sub(dq.len());
    dq.iter()
        .enumerate()
        .map(|(i, &v)| ((i + offset) as f64, v))
        .collect()
}
