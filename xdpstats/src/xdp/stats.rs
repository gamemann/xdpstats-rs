use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{collections::HashMap, io};

use anyhow::{Result, anyhow};

use aya::maps::{MapData, PerCpuArray, PerCpuValues};

use xdpstats_common::{StatType, StatVal};

use aya::util::nr_cpus;

use std::io::Write;

use crate::util::{format_byt, format_pkt};
use crate::xdp::base::XdpBase;

pub type StatsGlobal = Arc<Mutex<PerCpuArray<MapData, StatVal>>>;

#[derive(Debug, Default)]
pub struct StatEntry {
    pub cur: StatVal,
    pub prev: StatVal,
}

#[derive(Debug)]
pub struct Stats {
    pub entry: HashMap<StatType, StatEntry>,
    pub last_update: Instant,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            entry: HashMap::new(),
            last_update: Instant::now(),
        }
    }
}

impl XdpBase {
    pub fn stats_init(&mut self) -> Result<()> {
        let cpu_cnt = nr_cpus().map_err(|(_, error)| error)?;

        // We need to insert an empty structure into the stats map.
        let stat_val = StatVal::default();

        // We need to iterate over all stat types and insert the empty structure for each type.
        for stat_type in StatType::ALL {
            let stat_key = stat_type.clone().into();

            // Insert into the BPF map itself.
            let stat_vals = PerCpuValues::try_from(vec![stat_val; cpu_cnt])?;

            self.stats_map
                .lock()
                .map_err(|e| anyhow!("Failed to lock stats map: {e}"))?
                .set(stat_key, stat_vals, 0)
                .map_err(|e| {
                    anyhow!(
                        "Failed to initialize stats map for type: {:?}: {e}",
                        stat_type
                    )
                })?;

            // We'll also stuff our hash map now -_O_-
            let stat_entry = StatEntry::default();

            self.stats.entry.insert(stat_type.clone(), stat_entry);
        }

        Ok(())
    }

    pub fn stats_calc(&mut self, per_sec: bool) -> Result<()> {
        if per_sec {
            let now = Instant::now();
            let elapsed = (now - self.stats.last_update).as_secs_f64();

            if elapsed < 1.0 {
                // Not enough time has passed, keep current values.
                return Ok(());
            }

            // Snapshot raw values for all stat types first.
            let mut raws = HashMap::new();
            for stat in StatType::ALL {
                let raw = self
                    .stat_get_raw(stat.clone())
                    .map_err(|e| anyhow!("Failed to get raw stats: {e}"))?;
                raws.insert(stat.clone(), raw);
            }

            // Now calculate deltas for all types using the same elapsed window.
            for stat in StatType::ALL {
                if let (Some(raw), Some(entry)) = (raws.get(stat), self.stats.entry.get_mut(stat)) {
                    let delta_pkt = raw.pkt.saturating_sub(entry.prev.pkt);
                    let delta_byt = raw.byt.saturating_sub(entry.prev.byt);

                    entry.cur = StatVal {
                        pkt: (delta_pkt as f64 / elapsed) as u64,
                        byt: (delta_byt as f64 / elapsed) as u64,
                    };

                    entry.prev = *raw;
                }
            }

            // Update timestamp once after all stats processed.
            self.stats.last_update = now;
        } else {
            for stat in StatType::ALL {
                let stat_val = self
                    .stat_get_raw(stat.clone())
                    .map_err(|e| anyhow!("Failed to calculate stats: {e}"))?;

                if let Some(entry) = self.stats.entry.get_mut(stat) {
                    entry.cur = stat_val;
                }
            }
        }

        Ok(())
    }

    pub fn stat_get(&mut self, per_sec: bool, stat_type: StatType) -> Result<StatVal> {
        if per_sec {
            Ok(self
                .stats
                .entry
                .get(&stat_type)
                .map(|e| e.cur.clone())
                .unwrap_or_default())
        } else {
            self.stat_get_raw(stat_type)
        }
    }

    pub fn stat_get_raw(&mut self, stat_type: StatType) -> Result<StatVal> {
        let stats_key = stat_type as u32;

        let mut ret = StatVal::default();

        let stats = self
            .stats_map
            .lock()
            .map_err(|e| anyhow!("Failed to lock stats map: {e}"))?
            .get(&stats_key, 0)
            .map_err(|e| anyhow!("Failed to get stats from map: {e}"))?;

        for cpu_stats in stats.iter() {
            ret.pkt += cpu_stats.pkt;
            ret.byt += cpu_stats.byt;
        }

        Ok(ret)
    }

    pub fn stat_get_by_sec(&mut self, stat_type: StatType) -> Result<StatVal> {
        let stats_raw = self
            .stat_get_raw(stat_type.clone())
            .map_err(|e| anyhow!("Failed to get stats: {e}"))?;

        let now = Instant::now();
        let elapsed = (now - self.stats.last_update).as_secs_f64();

        if elapsed < 1.0 {
            return Ok(self
                .stats
                .entry
                .get(&stat_type)
                .map(|e| e.cur.clone())
                .unwrap_or_default());
        }

        let en = self
            .stats
            .entry
            .get_mut(&stat_type)
            .ok_or_else(|| anyhow!("Failed to get stats entry"))?;

        // Calculate delta from last raw snapshot.
        let delta_pkt = stats_raw.pkt.saturating_sub(en.prev.pkt);
        let delta_byt = stats_raw.byt.saturating_sub(en.prev.byt);

        let pkt_per_sec = (delta_pkt as f64 / elapsed) as u64;
        let byt_per_sec = (delta_byt as f64 / elapsed) as u64;

        // Update prev to current raw and timestamp.
        en.prev = stats_raw;
        self.stats.last_update = now;

        Ok(StatVal {
            pkt: pkt_per_sec,
            byt: byt_per_sec,
        })
    }

    pub fn get_all(&mut self, per_sec: bool) -> Result<HashMap<&StatType, StatVal>> {
        let mut all_stats = HashMap::new();

        for stat_type in StatType::ALL {
            let stat_val = self
                .stat_get(per_sec, stat_type.clone())
                .map_err(|e| anyhow!("Failed to get stats for type: {:?}: {e}", stat_type))?;

            all_stats.insert(stat_type, stat_val);
        }

        Ok(all_stats)
    }

    pub fn stats_display_pretty(&mut self, per_sec: bool, flush: bool) -> Result<()> {
        let stats_full = self
            .get_all(per_sec)
            .map_err(|e| anyhow!("Failed to retrieve all stats for display: {e}"))?;

        let stat_matched = stats_full
            .get(&StatType::MATCH)
            .ok_or_else(|| anyhow!("Failed to get MATCHED stats"))?;

        print!(
            "\r\x1b[1;34mMatched:\x1b[0m {} / {}  |  ",
            format_pkt(stat_matched.pkt as f64, per_sec),
            format_byt(stat_matched.byt as f64, per_sec)
        );

        let stat_error = stats_full
            .get(&StatType::ERROR)
            .ok_or_else(|| anyhow!("Failed to get ERROR stats"))?;

        print!(
            "\x1b[31mError:\x1b[0m {} / {}  |  ",
            format_pkt(stat_error.pkt as f64, per_sec),
            format_byt(stat_error.byt as f64, per_sec)
        );

        let stat_bad = stats_full
            .get(&StatType::BAD)
            .ok_or_else(|| anyhow!("Failed to get BAD stats"))?;

        print!(
            "\x1b[1;33mBad:\x1b[0m {} / {}  |  ",
            format_pkt(stat_bad.pkt as f64, per_sec),
            format_byt(stat_bad.byt as f64, per_sec)
        );

        let stat_drop = stats_full
            .get(&StatType::DROP)
            .ok_or_else(|| anyhow!("Failed to get DROP stats"))?;

        print!(
            "\x1b[1;31mDropped:\x1b[0m {} / {}  |  ",
            format_pkt(stat_drop.pkt as f64, per_sec),
            format_byt(stat_drop.byt as f64, per_sec)
        );

        let stat_pass = stats_full
            .get(&StatType::PASS)
            .ok_or_else(|| anyhow!("Failed to get PASS stats"))?;

        print!(
            "\x1b[1;32mPassed:\x1b[0m {} / {}  |  ",
            format_pkt(stat_pass.pkt as f64, per_sec),
            format_byt(stat_pass.byt as f64, per_sec)
        );

        if flush {
            io::stdout()
                .flush()
                .map_err(|e| anyhow!("Failed to flush stdout: {e}"))?;
        }

        Ok(())
    }
}

pub fn get_stats_rel(cur: StatVal, prev: StatVal) -> StatVal {
    StatVal {
        pkt: cur.pkt - prev.pkt,
        byt: cur.byt - prev.byt,
    }
}
