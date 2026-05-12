#![no_std]
#![no_main]

mod ctx;

mod stats;
mod util;

mod protocol;

mod fib;
mod tx;

use stats::inc_stats;
use util::ptr_at;

use xdpstats_common::{StatType, StatVal, TARGET_PORT, config::TARGET_PROTOCOL};

use aya_ebpf::{
    bindings::xdp_action::{self},
    macros::{map, xdp},
    maps::PerCpuArray,
    programs::XdpContext,
};

use xdp_action::{XDP_ABORTED, XDP_DROP, XDP_PASS};

use network_types::{
    eth::{EthHdr, EtherType},
    icmp::Icmpv4Hdr,
    ip::{IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

use crate::{ctx::Context, protocol::Protocol};

use cfg_if::cfg_if;

#[map]
static MAP_STATS: PerCpuArray<StatVal> = PerCpuArray::with_max_entries(StatType::STATCNT as u32, 0);

#[cfg(feature = "afxdp")]
mod afxdp {
    use aya_ebpf::macros::map;
    use aya_ebpf::maps::XskMap;
    use xdpstats_common::config::MAX_CPUS;

    #[map]
    pub static MAP_XSK: XskMap = XskMap::with_max_entries(MAX_CPUS as u32, 0);
}

#[inline(always)]
fn exit_prog(pkt_len: u16, stats_type: StatType, ret: xdp_action::Type) -> u32 {
    inc_stats(stats_type, pkt_len as u64);

    ret
}

#[xdp]
pub fn xdp_stats_pass_simple(_ctx: XdpContext) -> u32 {
    XDP_PASS
}

#[xdp]
pub fn xdp_stats_drop_simple(_ctx: XdpContext) -> u32 {
    XDP_DROP
}

#[xdp]
pub fn xdp_stats_pass_simple_stats(ctx: XdpContext) -> u32 {
    let pkt_len = (ctx.data_end() - ctx.data()) as u16;

    inc_stats(StatType::PASS, pkt_len as u64);

    XDP_PASS
}

#[xdp]
pub fn xdp_stats_drop_simple_stats(ctx: XdpContext) -> u32 {
    let pkt_len = (ctx.data_end() - ctx.data()) as u16;

    inc_stats(StatType::DROP, pkt_len as u64);

    XDP_DROP
}

#[xdp]
pub fn xdp_stats(ctx: XdpContext) -> u32 {
    // We can retrieve the total packet length by subtracting data from data_end.
    let pkt_len = (ctx.data_end() - ctx.data()) as u16;

    // We need to initialize the ethernet header and check.
    let eth: *const EthHdr = match unsafe { ptr_at(&ctx, 0) } {
        Ok(eth) => eth,
        Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
    };

    // We need to pass packets to the Linux network stack if they aren't an IPv4 packet.
    match unsafe { (*eth).ether_type() } {
        Ok(EtherType::Ipv4) => {}
        _ => return exit_prog(pkt_len, StatType::PASS, XDP_PASS),
    }

    // Initialize and check IPv4 header.
    let iph: *const Ipv4Hdr = match unsafe { ptr_at(&ctx, EthHdr::LEN) } {
        Ok(iph) => iph,
        Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
    };

    // We only support UDP packets for best performance.
    match unsafe { (*iph).proto } {
        TARGET_PROTOCOL => {}
        _ => return exit_prog(pkt_len, StatType::PASS, XDP_PASS),
    }

    // Retrieve IP header length.
    // NOTE: Dynamically retrieving the IP header length and doing a check results in a bad packet. This is likely due to the verifier, but I don't have this issue in C. IPv4 header should be 20 bytes anyways though ¯\_(ツ)_/¯
    //let ip_len = (unsafe { (*iph).ihl() } as usize) * 4;
    let ip_len = Ipv4Hdr::LEN;

    let start_proto: *const u8 = match unsafe { (*iph).proto } {
        x if x == IpProto::Tcp as u8 => {
            match unsafe { ptr_at::<TcpHdr>(&ctx, EthHdr::LEN + ip_len) } {
                Ok(ptr) => ptr as *const u8,
                Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
            }
        }
        x if x == IpProto::Udp as u8 => {
            match unsafe { ptr_at::<UdpHdr>(&ctx, EthHdr::LEN + ip_len) } {
                Ok(ptr) => ptr as *const u8,
                Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
            }
        }
        x if x == IpProto::Icmp as u8 => {
            match unsafe { ptr_at::<Icmpv4Hdr>(&ctx, EthHdr::LEN + ip_len) } {
                Ok(ptr) => ptr as *const u8,
                Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
            }
        }
        _ => return exit_prog(pkt_len, StatType::PASS, XDP_PASS),
    };

    // Initialize protocol.
    let proto = Protocol::new(unsafe { (*iph).proto }, start_proto);

    // Create context now.
    #[allow(unused)]
    let mut ctx = Context {
        xdp_ctx: ctx,
        eth: eth,
        iph: iph,
        proto: proto,
    };

    // Retrieve destination port and check.
    // If the target port isn't right, pass to network stack.
    if TARGET_PORT != 0
        && let Some(port) = ctx.proto.get_dst_port()
    {
        if port != TARGET_PORT {
            return exit_prog(pkt_len, StatType::PASS, XDP_PASS);
        }
    }

    cfg_if! {
        if #[cfg(feature = "afxdp")]
        {
            use aya_ebpf::helpers;
            use xdp_action::XDP_REDIRECT;

            // Check if we're doing TX.
            #[cfg(feature = "tx")]
            {
                use crate::tx::do_tx;

                // Now perform TX actions ensuring we don't get an error (1).
                if do_tx(&mut ctx, pkt_len) != 0 {
                    return exit_prog(pkt_len, StatType::DROP, XDP_DROP);
                }
            }

            let queue_id = ctx.xdp_ctx.rx_queue_index();

            // If we're in AF_XDP mode, we need to forward the packet to the correct socket.
            match afxdp::MAP_XSK.redirect(queue_id, 0) {
                Ok(ret) => return ret, // We don't increment stats here since it's done in the AF_XDP socket processing function.
                Err(_) => {
                    return exit_prog(pkt_len, StatType::ERROR, XDP_DROP);
                }
            }
        } else if #[cfg(feature = "tx")]
        {
            use crate::tx::do_tx;
            use xdp_action::XDP_TX;

            // Now perform TX actions ensuring we don't get an error (1).
            if do_tx(&mut ctx, pkt_len) != 0 {
                return exit_prog(pkt_len, StatType::DROP, XDP_DROP);
            }

            return exit_prog(pkt_len, StatType::MATCH, XDP_TX);
        } else {
             exit_prog(pkt_len, StatType::MATCH, XDP_DROP)
        }
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
