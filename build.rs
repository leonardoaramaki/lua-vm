// Compiles the Lua part of the standard library into bytecode that the VM embeds.
fn main() {
    println!("cargo:rerun-if-changed=src/prelude.lua");
    let bytecode = luac::compile_file("src/prelude.lua").expect("src/prelude.lua must compile");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("prelude.luac");
    std::fs::write(out, bytecode).unwrap();
}
