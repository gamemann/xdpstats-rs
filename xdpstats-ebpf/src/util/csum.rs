use aya_ebpf::helpers::bpf_csum_diff;
use network_types::ip::Ipv4Hdr;

/// Helper function for folding a 64-bit checksum into 16 bits by adding carries.
///
/// # Arguments
/// * `csum` - The 64-bit checksum to fold.
///
/// # Returns
/// The folded 16-bit checksum.
#[inline(always)]
fn fold_csum(mut csum: u64) -> u16 {
    csum = (csum & 0xffff) + (csum >> 16);
    csum = (csum & 0xffff) + (csum >> 16);
    csum as u16
}

/// Calculates the IPv4 header checksum for the given IP header and returns it in network byte order.
///
/// # Arguments
/// * `iph` - A pointer to the IPv4 header for which to calculate the checksum.
///
/// # Returns
/// The calculated checksum in network byte order.
#[inline(always)]
pub unsafe fn csum_calc_ip(iph: *mut Ipv4Hdr) -> u16 {
    unsafe {
        (*iph).check = [0, 0];
    }

    let csum = unsafe { bpf_csum_diff(core::ptr::null_mut(), 0, iph as *mut u32, 20, 0) };

    !fold_csum(csum as u64)
}

/// Incrementally update a checksum after changing a 16-bit field (e.g. port).
///
/// # Arguments
/// * `old_csum` - The old checksum.
/// * `old_val` - The old 16-bit value.
/// * `new_val` - The new 16-bit value.
///
/// # Returns
/// The updated checksum.
#[inline(always)]
pub fn csum_update_u16(old_csum: u16, old_val: u16, new_val: u16) -> u16 {
    let mut csum = !old_csum as u32;

    csum += !old_val as u32;
    csum += new_val as u32;

    // Fold carries
    csum = (csum & 0xffff) + (csum >> 16);
    csum = (csum & 0xffff) + (csum >> 16);

    !(csum as u16)
}

/// Incrementally update a checksum after changing a 32-bit field (e.g. IP address).
///
/// # Arguments
/// * `old_csum` - The old checksum.
/// * `old_val` - The old 32-bit value.
/// * `new_val` - The new 32-bit value.
///
/// # Returns
/// The updated checksum.
#[inline(always)]
pub fn csum_update_u32(old_csum: u16, old_val: u32, new_val: u32) -> u16 {
    let mut csum = !old_csum as u32;

    // Need to treat as two 16-bit halves and fold carries as we go to avoid overflow.
    csum += !(old_val >> 16) as u32;
    csum += !(old_val & 0xffff) as u32;

    csum += (new_val >> 16) as u32;
    csum += (new_val & 0xffff) as u32;

    csum = (csum & 0xffff) + (csum >> 16);
    csum = (csum & 0xffff) + (csum >> 16);

    !(csum as u16)
}
