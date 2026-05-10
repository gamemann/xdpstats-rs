use crate::xdp::base::XdpBase;

use anyhow::{Result, anyhow};

impl XdpBase {
    pub fn load(&mut self) -> Result<()> {
        let sec_name = self.sec_name.clone();

        // Now attempt to load our XDP program.
        let prog = self
            .get(&sec_name)
            .map_err(|e| anyhow!("Failed to retrieve XDP program section for loading: {e}"))?;

        prog.load()
            .map_err(|e| anyhow!("Failed to load XDP program: {e}"))?;

        Ok(())
    }
}
