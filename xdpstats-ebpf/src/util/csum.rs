use aya_ebpf::{helpers::bpf_csum_diff, programs::XdpContext};
use network_types::{eth::EthHdr, ip::Ipv4Hdr};

#[inline(always)]
fn fold_csum(mut csum: u64) -> u16 {
    csum = (csum & 0xffff) + (csum >> 16);
    csum = (csum & 0xffff) + (csum >> 16);
    csum as u16
}

#[inline(always)]
pub unsafe fn csum_calc_ip(iph: *mut Ipv4Hdr) -> u16 {
    unsafe {
        (*iph).check = [0, 0];
    }

    let csum = unsafe { bpf_csum_diff(core::ptr::null_mut(), 0, iph as *mut u32, 20, 0) };

    !fold_csum(csum as u64)
}
/// Incrementally update a checksum after changing a 16-bit field.
/// RFC 1624: HC' = ~(~HC + ~m + m')
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
#[inline(always)]
pub fn csum_update_u32(old_csum: u16, old_val: u32, new_val: u32) -> u16 {
    let mut csum = !old_csum as u32;
    csum += !(old_val >> 16) as u32;
    csum += !(old_val & 0xffff) as u32;
    csum += (new_val >> 16) as u32;
    csum += (new_val & 0xffff) as u32;
    csum = (csum & 0xffff) + (csum >> 16);
    csum = (csum & 0xffff) + (csum >> 16);
    !(csum as u16)
}
