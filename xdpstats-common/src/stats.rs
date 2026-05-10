#[repr(C)]
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum StatType {
    PASS,
    DROP,
    BAD,
    MATCH,
    ERROR,
    STATCNT,
}

impl StatType {
    pub const ALL: &'static [StatType] = &[
        StatType::PASS,
        StatType::DROP,
        StatType::BAD,
        StatType::MATCH,
        StatType::ERROR,
    ];
}

impl From<StatType> for u32 {
    fn from(stat_type: StatType) -> Self {
        match stat_type {
            StatType::PASS => 0,
            StatType::DROP => 1,
            StatType::BAD => 2,
            StatType::MATCH => 3,
            StatType::ERROR => 4,

            StatType::STATCNT => 5,
        }
    }
}

#[repr(C)]
#[derive(Copy, Default, Debug, Clone)]
pub struct StatVal {
    pub pkt: u64,
    pub byt: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for StatVal {}
