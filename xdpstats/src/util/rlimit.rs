use anyhow::{Result, anyhow};

use libc::{RLIM_INFINITY, RLIMIT_MEMLOCK, rlimit, setrlimit};

pub fn raise_rlimit() -> Result<()> {
    let rlim = rlimit {
        rlim_cur: RLIM_INFINITY,
        rlim_max: RLIM_INFINITY,
    };

    let ret = unsafe { setrlimit(RLIMIT_MEMLOCK, &rlim) };

    if ret != 0 {
        Err(anyhow!("Failed to raise rlimit: {}", ret))
    } else {
        Ok(())
    }
}
