// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let plan = day_build::plan_resources(&root).expect("shared game resources");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("day_resources.rs"), day_build::render(&plan))
        .expect("resource accessors");
    println!("cargo:rerun-if-changed=../resource");
}
