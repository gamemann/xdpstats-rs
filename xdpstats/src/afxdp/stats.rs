use anyhow::{Result, anyhow, bail};
use aya::maps::{MapData, PerCpuArray, PerCpuValues};
use xdpstats_common::{StatType, StatVal};

pub struct AfxdpStats {
    pub map: PerCpuArray<MapData, StatVal>,
    core_id: u32,
}

impl AfxdpStats {
    pub fn new(map: PerCpuArray<MapData, StatVal>, core_id: u32) -> Self {
        Self { map, core_id }
    }

    #[inline(always)]
    pub fn inc(&mut self, stat_type: StatType, length: u64) -> Result<()> {
        let key = stat_type.clone() as u32;

        // Retrieve current stats for the given type.
        let map = self
            .map
            .get(&key, 0)
            .map_err(|e| anyhow!("Failed to get stats from map: {}", e))?;

        let mut vals: Vec<StatVal> = map.iter().copied().collect();

        let val = vals.get_mut(self.core_id as usize).ok_or_else(|| {
            anyhow!(
                "Failed to get stats for core {} and stat type {:?}",
                self.core_id,
                stat_type
            )
        })?;

        // We'll want the update the packet and byte counts for this core ID.
        val.pkt += 1;
        val.byt += length;

        // We need to reconstruct the PerCpuValues struct from the updated values.
        let vals_updated = PerCpuValues::try_from(vals)
            .map_err(|e| anyhow!("Failed to reconstruct PerCpuValues: {}", e))?;

        // Now we must save the entry again.
        match self.map.set(key, vals_updated, 0) {
            Ok(_) => (),
            Err(e) => bail!("Failed to update stats in map: {}", e),
        }

        Ok(())
    }
}
