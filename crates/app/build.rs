//! Compiles the Swift bridge to the Foundation Models framework into a static
//! library and links it with the Swift runtime of macOS (STANDARD §11.1).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "swift/FoundationModels.swift";
const LIBRARY: &str = "smv_foundation_models";

fn main() {
    println!("cargo:rerun-if-changed={SOURCE}");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("set by cargo"));
    let sdk = xcrun(&["--sdk", "macosx", "--show-sdk-path"]);
    let swiftc = xcrun(&["--find", "swiftc"]);
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("unsupported architecture {other:?}"),
    };
    let deployment = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "26.0".into());

    let library = out.join(format!("lib{LIBRARY}.a"));
    let status = Command::new(&swiftc)
        .args(["-parse-as-library", "-emit-library", "-static", "-O"])
        .args(["-swift-version", "5", "-module-name", "SMVFoundationModels"])
        .args(["-target", &format!("{arch}-apple-macosx{deployment}")])
        .args(["-sdk", &sdk])
        .arg("-o")
        .arg(&library)
        .arg(manifest.join(SOURCE))
        .status()
        .expect("cannot run swiftc");
    assert!(status.success(), "swiftc failed to compile {SOURCE}");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static={LIBRARY}");
    // The Swift runtime is part of macOS: link against the stubs in the SDK
    // and the compatibility libraries of the toolchain, and load it from
    // /usr/lib/swift at run time.
    println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    let toolchain = Path::new(&swiftc)
        .parent()
        .and_then(Path::parent)
        .expect("swiftc lives in <toolchain>/usr/bin");
    println!(
        "cargo:rustc-link-search=native={}",
        toolchain.join("lib/swift/macosx").display()
    );
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-lib=framework=FoundationModels");
}

fn xcrun(args: &[&str]) -> String {
    let output = Command::new("xcrun")
        .args(args)
        .output()
        .expect("cannot run xcrun");
    assert!(output.status.success(), "xcrun {args:?} failed");
    String::from_utf8(output.stdout)
        .expect("xcrun prints UTF-8")
        .trim()
        .to_owned()
}
