use anyhow::{Context, Result, anyhow};
use aya::{
    Ebpf,
    programs::{Xdp as AyaXdp, xdp::XdpLinkId},
};
use xdpstats_common::PATH_ELF_FILE;

use crate::xdp::stats::Stats;

pub struct XdpBase {
    pub prog_bpf: Ebpf,
    pub link_ids: Vec<XdpLinkId>,

    pub sec_name: String,

    pub stats: Stats,
}

pub type Xdp = XdpBase;

impl XdpBase {
    pub fn new(sec_name: &str) -> Result<Self> {
        // We need to build our ELF path to load with eBPF.
        let elf_path = format!("{}/{}", env!("OUT_DIR"), PATH_ELF_FILE);

        // Attempt to load our eBPF program.
        let prog_bpf =
            Ebpf::load_file(elf_path).map_err(|e| anyhow!("Failed to load eBPF program: {e}"))?;

        Ok(Self {
            prog_bpf,
            link_ids: Vec::new(),
            sec_name: sec_name.to_string(),
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
