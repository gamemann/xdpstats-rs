use aya_ebpf::programs::XdpContext;
use network_types::{eth::EthHdr, ip::Ipv4Hdr};

pub struct Context {
    pub xdp_ctx: XdpContext,
    pub eth: *const EthHdr,
    pub iph: *const Ipv4Hdr,

    pub src_port: u16,
    pub dst_port: u16,
}
