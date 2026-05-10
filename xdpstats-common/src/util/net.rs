/// Swaps the source and destination MAC addresses in the Ethernet header.
///
/// # Arguments
/// * `src` - A mutable byte slice representing the source MAC address (6 bytes).
/// * `dst` - A mutable byte slice representing the destination MAC address (6 bytes).
#[inline(always)]
pub fn swap_eth(src: &mut [u8], dst: &mut [u8]) {
    for i in 0..6 {
        let tmp = src[i];

        src[i] = dst[i];
        dst[i] = tmp;
    }
}

/// Swaps the source and destination IP addresses in the IPv4 header.
///
/// # Arguments
/// * `src` - A mutable byte slice representing the source IP address (4 bytes).
/// * `dst` - A mutable byte slice representing the destination IP address (4 bytes).
#[inline(always)]
pub fn swap_ip(src: &mut [u8], dst: &mut [u8]) {
    for i in 0..4 {
        let tmp = src[i];

        src[i] = dst[i];
        dst[i] = tmp;
    }
}

/// Swaps the source and destination ports in the TCP and UDP headers.
///
/// # Arguments
/// * `src` - A mutable byte slice representing the source port (2 bytes).
/// * `dst` - A mutable byte slice representing the destination port (2 bytes).
#[inline(always)]
pub fn swap_port(src: &mut [u8], dst: &mut [u8]) {
    for i in 0..2 {
        let tmp = src[i];

        src[i] = dst[i];
        dst[i] = tmp;
    }
}
