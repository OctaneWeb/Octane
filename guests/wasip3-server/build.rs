// Link against wasi-sdk's WASIp3 libc and its `crt1-reactor.o`. rustc has
// no self-contained sysroot for this tier 3 target, and for a cdylib it would
// otherwise take crt1-reactor.o from there. That object sets up the per-task
// context: without it wasm-ld doesn't generate the
// `__wasm_get_stack_pointer`/`__wasm_set_stack_pointer` imports that every
// object built for cooperative threading needs.
use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("p3") {
        return;
    }
    let sdk = env::var("WASI_SDK_PATH")
        .expect("set WASI_SDK_PATH to the wasi-sdk install (34 or newer has WASIp3)");
    let libc = PathBuf::from(sdk).join("share/wasi-sysroot/lib/wasm32-wasip3");
    assert!(libc.is_dir(), "{} is missing, is wasi-sdk 34+?", libc.display());
    println!("cargo:rustc-link-search=native={}", libc.display());

    let crt = libc.join("crt1-reactor.o");
    assert!(crt.is_file(), "{} is missing", crt.display());
    println!("cargo:rustc-link-arg={}", crt.display());
}
