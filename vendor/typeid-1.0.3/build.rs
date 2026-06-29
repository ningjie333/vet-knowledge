use std::env;
use std::process::Command;
use std::str;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let compiler = match rustc_minor_version() {
        Some(compiler) => compiler,
        None => return,
    };

    if compiler >= 80 {
        println!("cargo:rustc-check-cfg=cfg(no_const_type_id)");
    }

    if compiler < 61 {
        // Function pointer casting in const fn.
        // https://blog.rust-lang.org/2022/05/19/Rust-1.61.0.html#more-capabilities-for-const-fn
        println!("cargo:rustc-cfg=no_const_type_id");
    }
}

fn rustc_minor_version() -> Option<u32> {
    let rustc = env::var_os("RUSTC")?;
    // WORKAROUND: std::process::Command::output() panics on Windows when the
    // user path contains non-ASCII characters (e.g., Chinese username). The
    // panic is at `library/std/src/sys/process/mod.rs:65` — `res.unwrap()` on
    // `Err(io::Error { code: 0, kind: Uncategorized })`. Use spawn + read_to_end
    // + wait instead, deliberately ignoring read_to_end errors (which still
    // leave valid data in the buffer).
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
