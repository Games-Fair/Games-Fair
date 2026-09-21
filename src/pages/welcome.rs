// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception

use day::prelude::*;

/// The opening screen: the app mark, a greeting, and markdown prose from the translation
/// (https://daybrite.dev/docs/resources). Its links open in the browser.
pub(crate) fn welcome_page() -> impl Piece {
    column((
        spacer(),
        vector(crate::res::vectors::app_mark)
            .frame(132.0, 132.0)
            .corner_radius(30.0)
            .id("welcome-mark"),
        label(crate::res::str::welcome_title())
            .font(Font::LargeTitle)
            .align(TextAlign::Center)
            .id("welcome-title"),
        label(crate::res::str::welcome_body())
            .markdown()
            .on_link(|target| {
                // Desktop Settings opens a separate window rather than a nav section.
                if target == "#settings" && crate::has_menu_bar() {
                    day::open_preferences();
                } else {
                    open_link(target);
                }
            })
            .align(TextAlign::Center)
            .max_width(460.0)
            .id("welcome-body"),
        spacer(),
    ))
    .spacing(20.0)
    .align(HAlign::Center)
    // Fill the pane first, so centering has room to work.
    .grow()
    .padding(24.0)
}
