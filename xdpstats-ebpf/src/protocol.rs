use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::PerCpuArray,
    programs::XdpContext,
};

use network_types::{icmp::Icmpv4Hdr, ip::IpProto, tcp::TcpHdr, udp::UdpHdr};

use crate::util::csum::{csum_update_u16, csum_update_u32};

pub enum Protocol {
    Tcp(*const TcpHdr),
    Udp(*const UdpHdr),
    Icmp,
}

impl Protocol {
    /// Initiates a new protocol based off of the provided protocol number and data pointer.
    ///
    /// # Arguments
    /// * `proto` - The protocol number (e.g. TCP, UDP).
    /// * `data` - A pointer to the start of the protocol header (e.g. TCP or UDP header).
    ///
    /// # Returns
    /// A new `Protocol` enum instance with the appropriate type and data pointer.
    #[inline(always)]
    pub fn new(proto: u8, data: *const u8) -> Self {
        match proto {
            x if x == IpProto::Tcp as u8 => Protocol::Tcp(data as *const TcpHdr),
            x if x == IpProto::Udp as u8 => Protocol::Udp(data as *const UdpHdr),
            _ => Protocol::Icmp,
        }
    }

    /// Retrieves the source port from the protocol header if it is TCP or UDP.
    ///
    /// # Returns
    /// An `Option<u16>` containing the source port if the protocol is TCP or UDP, or `None` if it is ICMP or an unsupported protocol.
    #[inline(always)]
    pub fn get_src_port(&self) -> Option<u16> {
        match self {
            Protocol::Tcp(tcp) => Some(unsafe { u16::from_be_bytes((**tcp).source) }),
            Protocol::Udp(udp) => Some(unsafe { (**udp).src_port() }),
            _ => None,
        }
    }

    /// Retrieves the destination port from the protocol header if it is TCP or UDP.
    ///
    /// # Returns
    /// An `Option<u16>` containing the destination port if the protocol is TCP or UDP, or `None` if it is ICMP or an unsupported protocol.
    #[inline(always)]
    pub fn get_dst_port(&self) -> Option<u16> {
        match self {
            Protocol::Tcp(tcp) => Some(unsafe { u16::from_be_bytes((**tcp).dest) }),
            Protocol::Udp(udp) => Some(unsafe { (**udp).dst_port() }),
            _ => None,
        }
    }

    /// Swaps the source and destination ports in the protocol header if it is TCP or UDP.
    #[inline(always)]
    pub fn swap_ports(&self) {
        match self {
            Protocol::Tcp(tcp) => unsafe {
                let tcp = &mut *(*tcp as *const TcpHdr as *mut TcpHdr);

                let tmp = tcp.source;
                tcp.source = tcp.dest;
                tcp.dest = tmp;
            },
            Protocol::Udp(udp) => unsafe {
                let udp = &mut *(*udp as *const UdpHdr as *mut UdpHdr);
                let tmp = udp.src_port();
                udp.set_src_port(udp.dst_port());
                udp.set_dst_port(tmp);
            },
            _ => {}
        }
    }

    /// Calculates the new checksum for the protocol header after swapping IP addresses and updates it accordingly if the protocol is TCP or UDP.
    ///
    /// # Arguments
    /// * `old_src_ip` - The old source IP address in host byte order.
    /// * `old_dst_ip` - The old destination IP address in host byte order.
    #[inline(always)]
    pub fn calc_csum(&self, old_src_ip: u32, old_dst_ip: u32) {
        match self {
            Protocol::Tcp(tcp) => unsafe {
                let tcp = &mut *(*tcp as *mut TcpHdr);

                // Store old checksum.
                let old_csum = u16::from_be_bytes(tcp.check);

                // Update source and destination swaps.
                let csum = csum_update_u32(old_csum, old_src_ip, old_dst_ip);
                let csum = csum_update_u32(csum, old_dst_ip, old_src_ip);

                // Update checksum.
                tcp.check = u16::to_be_bytes(csum);
            },
            Protocol::Udp(udp) => unsafe {
                let udp = &mut *(*udp as *mut UdpHdr);

                // Store old checksum.
                let old_csum = u16::from_be_bytes(udp.check);

                // Update source and destination swaps.
                let csum = csum_update_u32(old_csum, old_src_ip, old_dst_ip);
                let csum = csum_update_u32(csum, old_dst_ip, old_src_ip);

                // Update checksum.
                udp.check = u16::to_be_bytes(csum);
            },
            _ => {}
        }
    }
}
