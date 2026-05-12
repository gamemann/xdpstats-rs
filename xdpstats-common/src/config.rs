use network_types::ip::IpProto;

/* CONFIG OPTIONS */
/* -------------------------------- */
// The target protocol to match.
// You may use IpProto::Tcp, IpProto:Icmp, etc.
pub const TARGET_PROTOCOL: u8 = IpProto::Udp as u8;

// The target port to match packets on.
// Set this to 0 for no port matching.
pub const TARGET_PORT: u16 = 8080;

// The path to the ELF file to load with eBPF.
// Relative to $OUT_DIR env var, but you shouldn't need to change this.
pub const PATH_ELF_FILE: &str = "xdpstats";

// Max CPUs supported by the program.
pub const MAX_CPUS: usize = 256;
/* -------------------------------- */
/* CONFIG OPTIONS END */
