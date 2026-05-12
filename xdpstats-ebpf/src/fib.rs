use aya_ebpf::{
    EbpfContext,
    bindings::{
        BPF_FIB_LKUP_RET_SUCCESS,
        xdp_action::{self, XDP_DROP, XDP_TX},
    },
};
use aya_ebpf_bindings::{
    bindings::{
        bpf_fib_lookup as bpf_fib_lookup_params, bpf_fib_lookup__bindgen_ty_1,
        bpf_fib_lookup__bindgen_ty_2, bpf_fib_lookup__bindgen_ty_3, bpf_fib_lookup__bindgen_ty_4,
        bpf_fib_lookup__bindgen_ty_5,
    },
    helpers::bpf_fib_lookup,
};
use network_types::eth::EthHdr;

use crate::ctx::Context;

const AF_INET: u8 = 2;
const AF_INET6: u8 = 10;

/// Performs a FIB lookup for the given context and updates the Ethernet header with the new MAC addresses if the lookup is successful.
///
/// # Arguments
/// * `ctx` - The context containing the packet information for the FIB lookup.
/// * `len` - The total length of the packet for the FIB lookup.
///
/// # Returns
/// * `XDP_TX` if the FIB lookup was successful and the Ethernet header was updated.
/// * `XDP_DROP` if the FIB lookup failed.
pub fn do_fib_lookup(ctx: &Context, len: u16) -> u32 {
    // Fill out the fib lookup parameters.
    // Take a look here for more info:
    // https://docs.rs/aya-ebpf-bindings/latest/aya_ebpf_bindings/bindings/struct.bpf_fib_lookup.html
    let mut fib_lookup = bpf_fib_lookup_params {
        family: AF_INET,
        l4_protocol: unsafe { (*ctx.iph).proto as u8 },
        sport: ctx.proto.get_src_port().unwrap_or(0),
        dport: ctx.proto.get_dst_port().unwrap_or(0),
        ifindex: ctx.xdp_ctx.ingress_ifindex() as u32,
        smac: unsafe { (*ctx.eth).src_addr },
        dmac: unsafe { (*ctx.eth).dst_addr },

        __bindgen_anon_1: bpf_fib_lookup__bindgen_ty_1 { tot_len: len },
        __bindgen_anon_2: bpf_fib_lookup__bindgen_ty_2 {
            tos: unsafe { (*ctx.iph).tos },
        },
        __bindgen_anon_3: bpf_fib_lookup__bindgen_ty_3 {
            ipv4_src: unsafe { u32::from_be_bytes((*ctx.iph).src_addr) },
        },
        __bindgen_anon_4: bpf_fib_lookup__bindgen_ty_4 {
            ipv4_dst: unsafe { u32::from_be_bytes((*ctx.iph).dst_addr) },
        },

        __bindgen_anon_5: bpf_fib_lookup__bindgen_ty_5 { tbid: 0 },
    };

    let lookup = unsafe {
        bpf_fib_lookup(
            ctx.xdp_ctx.as_ptr(),
            &mut fib_lookup,
            core::mem::size_of::<bpf_fib_lookup_params>() as i32,
            0,
        )
    };

    // Check if the lookup was successful.
    if lookup == BPF_FIB_LKUP_RET_SUCCESS as i64 {
        // Update the Ethernet header with the new MAC addresses.
        unsafe {
            let eth_mut = ctx.eth as *mut EthHdr;
            (*eth_mut).src_addr = fib_lookup.smac;
            (*eth_mut).dst_addr = fib_lookup.dmac;
        }

        return XDP_TX;
    }

    XDP_DROP
}
