/* CONFIG OPTIONS */
/* -------------------------------- */
// The target UDP Port to match packets on.
pub const TARGET_PORT: u16 = 8080;

// The path to the ELF file to load with eBPF.
// Relative to $OUT_DIR env var, but you shouldn't need to change this.
pub const PATH_ELF_FILE: &str = "xdpstats";

// Max CPUs supported by the program.
pub const MAX_CPUS: usize = 256;
/* -------------------------------- */
/* CONFIG OPTIONS END */
