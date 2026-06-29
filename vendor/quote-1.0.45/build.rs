use std::env;
use std::process::Command;
use std::str;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let Some(minor) = rustc_minor_version() else {
        return;
    };

    if minor >= 77 {
        println!("cargo:rustc-check-cfg=cfg(no_diagnostic_namespace)");
    }

    // Support for the `#[diagnostic]` tool attribute namespace
    // https://blog.rust-lang.org/2024/05/02/Rust-1.78.0.html#diagnostic-attributes
    if minor < 78 {
        println!("cargo:rustc-cfg=no_diagnostic_namespace");
    }
}

fn rustc_minor_version() -> Option<u32> {
    let rustc = env::var_os("RUSTC")?;
    // WORKAROUND: std::process::Command::output() panics on Windows when the
    // user path contains non-ASCII characters (e.g., Chinese username). Use
    // spawn + read_to_end + wait instead, ignoring read_to_end errors.
    use std::io::Read;
    let mut cmd = Command::new(rustc);
    cmd.arg("--version");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    let mut buf = Vec::new();
    if let Some(out) = child.stdout.as_mut() {
        let _ = out.read_to_end(&mut buf);
    }
    let _ = child.wait();
    let version = str::from_utf8(&buf).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    pieces.next()?.parse().ok()
}
