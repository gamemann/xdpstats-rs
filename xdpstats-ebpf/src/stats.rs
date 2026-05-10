use xdpstats_common::StatType;

use crate::MAP_STATS;

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
