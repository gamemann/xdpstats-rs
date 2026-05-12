use crate::{ctx::Context, util::csum::csum_calc_ip};

use cfg_if::cfg_if;

use network_types::{eth::EthHdr, ip::Ipv4Hdr};

use aya_ebpf::bindings::xdp_action::XDP_TX;

#[inline(always)]
pub fn is_tx() -> u8 {
    cfg_if! {
        if #[cfg(feature = "tx")] {
            1
        } else {
            0
        }
    }
}

#[inline(always)]
pub fn do_tx(ctx: &mut Context, pkt_len: u16) -> u8 {
    cfg_if! {
        if #[cfg(feature = "tx_fib")] {
                // Perform FIB lookup and update MAC addresses now.
                use crate::fib::do_fib_lookup;

                match do_fib_lookup(&ctx, pkt_len) {
                    XDP_TX => {}
                    _ => {
                        return 1;
                    }
                }

        } else {
                use xdpstats_common::util::net::{swap_eth, swap_ip};
                use crate::util::csum::{csum_update_u32};

                // Store old IPs for checksum updates later.
                let old_src = u32::from_be_bytes(unsafe { (*ctx.iph).src_addr });
                let old_dst = u32::from_be_bytes(unsafe { (*ctx.iph).dst_addr });

                // Retrieve mutable pointers to the Ethernet and IP headers.
                let eth_mut = ctx.eth as *mut EthHdr;
                let ip_mut = ctx.iph as *mut Ipv4Hdr;

                // Swap everything: MAC addresses, IP addresses, and ports.
                swap_eth(unsafe { &mut (*eth_mut).src_addr }, unsafe { &mut (*eth_mut).dst_addr });
                swap_ip(unsafe { &mut (*ip_mut).src_addr }, unsafe { &mut (*ip_mut).dst_addr });
                ctx.proto.swap_ports();

                // Calculate protocol checksum update.
                ctx.proto.calc_csum(old_src, old_dst);

                // Finally calculate IP header checksum update.
                unsafe {
                    (*ip_mut).check = csum_calc_ip(ip_mut).to_be_bytes();
                }
        }
    }

    0
}
