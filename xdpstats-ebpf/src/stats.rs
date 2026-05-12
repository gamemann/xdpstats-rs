use xdpstats_common::StatType;

use crate::MAP_STATS;

/// Performs a lookup on the stats map for the given stats type and increments the packet and byte counts accordingly.
///
/// # Arguments
/// * `stats_type` - The type of statistic to increment (e.g., PASS, DROP).
/// * `length` - The length of the packet to add to the byte count for the given statistic type.
#[inline(always)]
pub fn inc_stats(stats_type: StatType, length: u64) {
    let stats = match MAP_STATS.get_ptr_mut(stats_type as u32) {
        Some(stats) => unsafe { &mut *stats },
        None => {
            return;
        }
    };

    stats.pkt += 1;
    stats.byt += length;
}
