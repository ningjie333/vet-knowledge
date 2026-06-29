use std::env;
use std::ffi::OsString;
use std::process::{self, Command};
use std::str;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let rustc = rustc_minor_version().unwrap_or(u32::MAX);

    if rustc >= 80 {
        println!("cargo:rustc-check-cfg=cfg(exhaustive)");
        println!("cargo:rustc-check-cfg=cfg(zmij_no_select_unpredictable)");
    }

    if rustc < 88 {
        // https://doc.rust-lang.org/std/hint/fn.select_unpredictable.html
        println!("cargo:rustc-cfg=zmij_no_select_unpredictable");
    }
}

fn rustc_minor_version() -> Option<u32> {
    let rustc = cargo_env_var("RUSTC");
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

fn cargo_env_var(key: &str) -> OsString {
    env::var_os(key).unwrap_or_else(|| {
        eprintln!("Environment variable ${key} is not set during execution of build script");
        process::exit(1);
    })
}
