use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use aya::{
    Ebpf,
    maps::{MapData, PerCpuArray},
    programs::{Xdp as AyaXdp, xdp::XdpLinkId},
};
use xdpstats_common::{PATH_ELF_FILE, StatVal};

use crate::xdp::stats::{Stats, StatsGlobal};

pub struct XdpBase {
    pub prog_bpf: Ebpf,
    pub link_ids: Vec<XdpLinkId>,

    pub sec_name: String,

    pub stats_map: StatsGlobal,
    pub stats: Stats,
}

pub type Xdp = XdpBase;

impl XdpBase {
    pub fn new(sec_name: &str) -> Result<Self> {
        // We need to build our ELF path to load with eBPF.
        let elf_path = format!("{}/{}", env!("OUT_DIR"), PATH_ELF_FILE);

        // Attempt to load our eBPF program.
        let mut prog_bpf =
            Ebpf::load_file(elf_path).map_err(|e| anyhow!("Failed to load eBPF program: {e}"))?;

        // Before we can return, we need to retrieve the stats map.
        let prog: &mut AyaXdp = prog_bpf
            .program_mut(sec_name)
            .ok_or_else(|| anyhow!("Section not found"))?
            .try_into()
            .map_err(|e| anyhow!("Failed to convert program: {e}"))?;

        let stats_map = {
            let map = prog_bpf
                .take_map("MAP_STATS")
                .ok_or_else(|| anyhow!("Failed to retrieve stats map"))?;

            let map = PerCpuArray::<MapData, StatVal>::try_from(map)
                .map_err(|e| anyhow!("Failed to convert stats map: {e}"))?;

            Arc::new(Mutex::new(map))
        };

        Ok(Self {
            prog_bpf,
            link_ids: Vec::new(),
            sec_name: sec_name.to_string(),
            stats_map,
            stats: Stats::default(),
        })
    }

    pub fn get(&mut self, sec_name: &str) -> Result<&mut AyaXdp> {
        // Retrieve XDP mutable reference to the program section.
        let prog: &mut AyaXdp = self
            .prog_bpf
            .program_mut(sec_name)
            .ok_or_else(|| anyhow!("Section not found"))?
            .try_into()?;

        Ok(prog)
    }
}
