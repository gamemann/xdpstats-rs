use anyhow::{Context as _, anyhow};
use aya_build::{Package, Toolchain};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_AFXDP");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_TX");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_TX_FIB");

    let mut features: Vec<&str> = vec![];

    if cfg!(feature = "afxdp") {
        features.push("afxdp");
    }
    if cfg!(feature = "tx") || cfg!(feature = "tx_fib") {
        features.push("tx");
    }
    if cfg!(feature = "tx_fib") {
        features.push("tx_fib");
    }

    let cargo_metadata::Metadata { packages, .. } = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .context("MetadataCommand::exec")?;

    let ebpf_package = packages
        .into_iter()
        .find(|cargo_metadata::Package { name, .. }| name.as_str() == "xdpstats-ebpf")
        .ok_or_else(|| anyhow!("xdpstats-ebpf package not found"))?;

    let cargo_metadata::Package {
        name,
        manifest_path,
        ..
    } = ebpf_package;

    let ebpf_package = Package {
        name: name.as_str(),
        root_dir: manifest_path
            .parent()
            .ok_or_else(|| anyhow!("no parent for {manifest_path}"))?
            .as_str(),
        features: features.as_slice(),
        ..Default::default()
    };

    aya_build::build_ebpf([ebpf_package], Toolchain::default())
}
