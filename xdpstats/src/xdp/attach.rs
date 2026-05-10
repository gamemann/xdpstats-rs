use anyhow::{Context, Result, anyhow};
use aya::programs::{Xdp, XdpFlags};

use crate::xdp::base::XdpBase;

impl XdpBase {
    pub fn attach(&mut self, iface: &str, flags: XdpFlags) -> Result<()> {
        let sec_name = self.sec_name.clone();

        // Retrieve a mutable reference to our XDP program section.
        let prog: &mut Xdp = self
            .get(&sec_name)
            .map_err(|e| anyhow!("Failed to retrieve XDP program section: {e}"))?;

        // Attempt to attach our XDP program to the specified interface with the provided flags.
        let link = prog
            .attach(iface, flags)
            .map_err(|e| anyhow!("Failed to attach XDP program: {e}"))?;

        // Push the link ID to our list of attached links so we can cleanup later.
        self.link_ids.push(link);

        Ok(())
    }
}
