// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! What day-build does before this project compiles: the typed `res::` constants generated from
//! `resource/` (https://daybrite.dev/docs/resources), and the app title `src/lib.rs` embeds for
//! the home and window headings.
fn main() {
    day_build::prebuild_project().expect("day-build: prebuild");
    let title = day_build::app_title().expect("day-build: app title");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(
        out.join("app_title.rs"),
        format!("const APP_TITLE: &str = {title:?};\n"),
    )
    .expect("write app title");
}
