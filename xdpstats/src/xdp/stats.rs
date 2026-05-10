use std::time::{Duration, Instant};
use std::{collections::HashMap, io};

use anyhow::{Result, anyhow};

use aya::maps::{MapData, PerCpuArray, PerCpuValues};

use xdpstats_common::{StatType, StatVal};

use aya::util::nr_cpus;

use std::io::Write;

use crate::util::{format_byt, format_pkt};
use crate::xdp::base::XdpBase;

#[derive(Debug, Default)]
pub struct StatEntry {
    cur: StatVal,
    prev: StatVal,
}

#[derive(Debug)]
pub struct Stats {
    pub entry: HashMap<StatType, StatEntry>,
    last_update: Instant,
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

        // We need to retrieve the map.
        let mut stats_map = PerCpuArray::try_from(self.prog_bpf.map_mut("MAP_STATS").unwrap())
            .map_err(|e| anyhow!("Failed to retrieve stats map: {e}"))?;

        // We need to iterate over all stat types and insert the empty structure for each type.
        for stat_type in StatType::ALL {
            let stat_key = stat_type.clone().into();

            // Insert into the BPF map itself.
            let stat_vals = PerCpuValues::try_from(vec![stat_val; cpu_cnt])?;

            stats_map.set(stat_key, stat_vals, 0).map_err(|e| {
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
        let stat_types = StatType::ALL;

        for stat in stat_types {
            let stat_val = self
                .stat_get(per_sec, stat.clone())
                .map_err(|e| anyhow!("Failed to calculate stats for type: {:?}: {e}", stat))?;

            if let Some(entry) = self.stats.entry.get_mut(stat) {
                entry.cur = stat_val;
            }
        }

        Ok(())
    }

    pub fn stat_get(&mut self, per_sec: bool, stat_type: StatType) -> Result<StatVal> {
        if per_sec {
            self.stat_get_by_sec(stat_type)
        } else {
            self.stat_get_raw(stat_type)
        }
    }

    pub fn stat_get_raw(&mut self, stat_type: StatType) -> Result<StatVal> {
        let stats_key = stat_type as u32;

        let stats_map: PerCpuArray<&mut MapData, StatVal> =
            PerCpuArray::try_from(self.prog_bpf.map_mut("MAP_STATS").unwrap())
                .map_err(|e| anyhow!("Failed to retrieve stats map: {e}"))?;

        let mut ret = StatVal::default();

        let stats = stats_map
            .get(&stats_key, 0)
            .map_err(|e| anyhow!("Failed to get stats from map: {e}"))?;

        for cpu_stats in stats.iter() {
            ret.pkt += cpu_stats.pkt;
            ret.byt += cpu_stats.byt;
        }

        Ok(ret)
    }

    pub fn stat_get_by_sec(&mut self, stat_type: StatType) -> Result<StatVal> {
        // Retrieve current stats.
        let stats_raw = self
            .stat_get_raw(stat_type.clone())
            .map_err(|e| anyhow!("Failed to get stats: {e}"))?;

        let now = Instant::now();

        let stats = &mut self.stats;

        if now - stats.last_update > Duration::from_secs(1) {
            // We need to reset previous stats to current and update the timestamp before returning the raw stats.
            let cur = {
                let mut cur = StatVal::default();

                for stat_type_raw in StatType::ALL {
                    if let Some(entry) = stats.entry.get_mut(stat_type_raw) {
                        entry.prev = entry.cur.clone();

                        // Check if this is the current stat.
                        if stat_type == *stat_type_raw {
                            cur = entry.cur.clone();
                        }
                    }
                }

                cur
            };

            stats.last_update = now;

            return Ok(cur);
        }

        let en = stats
            .entry
            .get_mut(&stat_type)
            .ok_or_else(|| anyhow!("Failed to get previous stats"))?;

        let ret = get_stats_rel(stats_raw, en.prev);

        // Update previous stats before returning
        en.prev = stats_raw;

        Ok(ret)
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
    pub fn stats_display_pretty(&self, per_sec: bool, flush: bool) -> Result<()> {
        let stat_matched = self
            .stats
            .entry
            .get(&StatType::MATCH)
            .ok_or_else(|| anyhow!("Failed to get MATCHED stats"))?;

        print!(
            "\r\x1b[1;34mMatched:\x1b[0m {} / {}  |  ",
            format_pkt(stat_matched.cur.pkt as f64, per_sec),
            format_byt(stat_matched.cur.byt as f64, per_sec)
        );

        let stat_error = self
            .stats
            .entry
            .get(&StatType::ERROR)
            .ok_or_else(|| anyhow!("Failed to get ERROR stats"))?;

        print!(
            "\x1b[31mError:\x1b[0m {} / {}  |  ",
            format_pkt(stat_error.cur.pkt as f64, per_sec),
            format_byt(stat_error.cur.byt as f64, per_sec)
        );

        let stat_bad = self
            .stats
            .entry
            .get(&StatType::BAD)
            .ok_or_else(|| anyhow!("Failed to get BAD stats"))?;

        print!(
            "\x1b[1;33mBad:\x1b[0m {} / {}  |  ",
            format_pkt(stat_bad.cur.pkt as f64, per_sec),
            format_byt(stat_bad.cur.byt as f64, per_sec)
        );

        let stat_drop = self
            .stats
            .entry
            .get(&StatType::DROP)
            .ok_or_else(|| anyhow!("Failed to get DROP stats"))?;

        print!(
            "\x1b[1;31mDropped:\x1b[0m {} / {}  |  ",
            format_pkt(stat_drop.cur.pkt as f64, per_sec),
            format_byt(stat_drop.cur.byt as f64, per_sec)
        );

        let stat_pass = self
            .stats
            .entry
            .get(&StatType::PASS)
            .ok_or_else(|| anyhow!("Failed to get PASS stats"))?;

        print!(
            "\x1b[1;32mPassed:\x1b[0m {} / {}  |  ",
            format_pkt(stat_pass.cur.pkt as f64, per_sec),
            format_byt(stat_pass.cur.byt as f64, per_sec)
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
