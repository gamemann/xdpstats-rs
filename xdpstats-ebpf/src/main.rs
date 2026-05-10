#![no_std]
#![no_main]

mod ctx;

mod fib;
mod stats;
mod utils;

use stats::inc_stats;
use utils::ptr_at;

use xdpstats_common::{StatType, StatVal, TARGET_PORT};

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::PerCpuArray,
    programs::XdpContext,
};

use xdp_action::{XDP_ABORTED, XDP_DROP, XDP_PASS};

use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr},
    udp::UdpHdr,
};

use crate::ctx::Context;

#[map]
static MAP_STATS: PerCpuArray<StatVal> = PerCpuArray::with_max_entries(StatType::STATCNT as u32, 0);

#[cfg(feature = "afxdp")]
#[map]
static MAP_XSK: XskMap = XskMap::with_max_entries(MAX_CPUS as u32, 0);

#[inline(always)]
fn exit_prog(pkt_len: u32, stats_type: StatType, ret: xdp_action::Type) -> u32 {
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
    let pkt_len = (ctx.data_end() - ctx.data()) as u32;

    inc_stats(StatType::PASS, pkt_len as u64);

    XDP_PASS
}

#[xdp]
pub fn xdp_stats_drop_simple_stats(ctx: XdpContext) -> u32 {
    let pkt_len = (ctx.data_end() - ctx.data()) as u32;

    inc_stats(StatType::DROP, pkt_len as u64);

    XDP_DROP
}

#[xdp]
pub fn xdp_stats(ctx: XdpContext) -> u32 {
    // We can retrieve the total packet length by subtracting data from data_end.
    let pkt_len = (ctx.data_end() - ctx.data()) as u32;

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
        IpProto::Udp => {}
        _ => return exit_prog(pkt_len, StatType::PASS, XDP_PASS),
    }

    // Retrieve IP header length.
    // NOTE: Dynamically retrieving the IP header length and doing a check results in a bad packet. This is likely due to the verifier, but I don't have this issue in C. IPv4 header should be 20 bytes anyways though ¯\_(ツ)_/¯
    //let ip_len = (unsafe { (*iph).ihl() } as usize) * 4;
    let ip_len = Ipv4Hdr::LEN;

    // We need to retrieve the UDP header.
    let udph: *const UdpHdr = match unsafe { ptr_at(&ctx, EthHdr::LEN + ip_len) } {
        Ok(udph) => udph,
        Err(_) => return exit_prog(pkt_len, StatType::BAD, XDP_ABORTED),
    };

    // Create context now.
    let ctx = Context {
        xdp_ctx: ctx,
        eth: eth,
        iph: iph,
        src_port: unsafe { (*udph).src_port() },
        dst_port: unsafe { (*udph).dst_port() },
    };

    // Retrieve destination port and check.
    // If the target port isn't right, pass to network stack.
    if ctx.dst_port != TARGET_PORT {
        return exit_prog(pkt_len, StatType::PASS, XDP_PASS);
    }

    #[cfg(feature = "afxdp")]
    {
        // If we're in TX mode and we're using FIB, we need to perform the lookup now and change the next hop MAC address before forwarding to the AF_XDP sockets.

        use aya_ebpf::helpers;
        #[cfg(all(feature = "tx"))]
        {
            // Check for FIB lookup feature.
            #[cfg(feature = "tx_fib")]
            {
                // Perform FIB lookup and update MAC addresses now.
                use crate::fib::do_fib_lookup;
                match do_fib_lookup(&ctx, len) {
                    xdp_action::XDP_TX => {}
                    _ => {
                        return exit_prog(pkt_len, StatType::DROP, XDP_DROP);
                    }
                }
            }

            #[cfg(all(not(feature = "tx_fib")))]
            {
                use xdpstats_common::util::net::{swap_eth, swap_ip};

                // We'll be just redirecting the packet back out. So swap everything now.
                let eth_mut = ctx.eth as *mut EthHdr;

                unsafe {
                    swap_eth(eth_mut);
                }

                swap_ip(src, dst);
            }
        }

        // If we're in AF_XDP mode, we need to forward the packet to the correct socket.
        let cpu_id = helpers::bpf_get_smp_processor_id() as u32;

        match MAP_XSK.get_ptr_mut(cpu_id) {
            Some(xsk) => unsafe {
                if (*xsk).fd != 0 {
                    return exit_prog(pkt_len, StatType::FWD, XDP_REDIRECT);
                } else {
                    info!(
                        &ctx,
                        "No socket FD found in XSK map for CPU {}, dropping packet.", cpu_id
                    );

                    return exit_prog(pkt_len, StatType::FWD, XDP_DROP);
                }
            },
            None => {
                info!(
                    &ctx,
                    "Failed to retrieve socket FD from XSK map for CPU {}, dropping packet.",
                    cpu_id
                );

                return exit_prog(pkt_len, StatType::FWD, XDP_DROP);
            }
        }
    }

    exit_prog(pkt_len, StatType::MATCH, XDP_DROP)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
