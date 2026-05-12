use aya_ebpf::programs::XdpContext;
use network_types::{eth::EthHdr, ip::Ipv4Hdr};

use crate::protocol::Protocol;

pub struct Context {
    pub xdp_ctx: XdpContext,
    pub eth: *const EthHdr,
    pub iph: *const Ipv4Hdr,
    pub proto: Protocol,
}
