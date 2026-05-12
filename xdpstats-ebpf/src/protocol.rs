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
    #[inline(always)]
    pub fn new(proto: u8, data: *const u8) -> Self {
        match proto {
            x if x == IpProto::Tcp as u8 => Protocol::Tcp(data as *const TcpHdr),
            x if x == IpProto::Udp as u8 => Protocol::Udp(data as *const UdpHdr),
            _ => Protocol::Icmp,
        }
    }

    #[inline(always)]
    pub fn get_src_port(&self) -> Option<u16> {
        match self {
            Protocol::Tcp(tcp) => Some(unsafe { u16::from_be_bytes((**tcp).source) }),
            Protocol::Udp(udp) => Some(unsafe { (**udp).src_port() }),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn get_dst_port(&self) -> Option<u16> {
        match self {
            Protocol::Tcp(tcp) => Some(unsafe { u16::from_be_bytes((**tcp).dest) }),
            Protocol::Udp(udp) => Some(unsafe { (**udp).dst_port() }),
            _ => None,
        }
    }

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

    #[inline(always)]
    pub fn calc_csum(&self, old_src_ip: u32, old_dst_ip: u32) {
        match self {
            Protocol::Tcp(tcp) => unsafe {
                let tcp = &mut *(*tcp as *mut TcpHdr);

                // Store old checksum.
                let old_csum = u16::from_be_bytes(tcp.check);

                // Update for src IP becoming dst IP
                let csum = csum_update_u32(old_csum, old_src_ip, old_dst_ip);

                // Update for dst IP becoming src IP
                let csum = csum_update_u32(csum, old_dst_ip, old_src_ip);

                tcp.check = u16::to_be_bytes(csum);
            },
            Protocol::Udp(udp) => unsafe {
                let udp = &mut *(*udp as *mut UdpHdr);
                let old_csum = u16::from_be_bytes(udp.check);
                let csum = csum_update_u32(old_csum, old_src_ip, old_dst_ip);
                let csum = csum_update_u32(csum, old_dst_ip, old_src_ip);
                udp.check = u16::to_be_bytes(csum);
            },
            _ => {}
        }
    }
}
