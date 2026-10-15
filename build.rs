// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! What day-build does before this project compiles: the typed `res::` constants generated from
//! `resource/` (https://daybrite.dev/docs/resources), the app title `src/lib.rs` embeds for the
//! home and window headings, and the bundled fonts' license texts About shows.
fn main() {
    day_build::prebuild_project().expect("day-build: prebuild");
    let title = day_build::app_title().expect("day-build: app title");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(
        out.join("app_title.rs"),
        format!("const APP_TITLE: &str = {title:?};\n"),
    )
    .expect("write app title");
    std::fs::write(out.join("font_licenses.txt"), font_licenses()).expect("write font licenses");
}

/// The bundled fonts' license texts under resource/font-licenses/, in file-name order, one
/// after another: the About window's Licenses page. Each text opens with its font's copyright
/// line, so no heading is added.
fn font_licenses() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resource/font-licenses");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("resource/font-licenses")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();
    let mut out = String::new();
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(&path).expect("read font license");
        if !out.is_empty() {
            out.push_str("\n\n\n");
        }
        out.push_str(text.trim_end());
        out.push('\n');
    }
    out
}
