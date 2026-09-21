// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception

use crate::res;
use day::prelude::*;

/// The second section: an introduction and three links the platform opens in the browser.
/// Replace it with what your app's second section should be.
pub(crate) fn navigate_page() -> impl Piece {
    column((
        label(res::str::navigate_title())
            .font(Font::Title)
            .id("navigate-title"),
        label(res::str::navigate_body())
            .markdown()
            .on_link(open_link)
            .max_width(520.0)
            .id("navigate-body"),
        form((section((
            link(res::str::link_home(), "https://appfair.org").id("link-home"),
            link(res::str::link_docs(), "https://appfair.org/docs/").id("link-docs"),
            link(res::str::link_source(), "https://github.com/appfair").id("link-source"),
        ))
        .title(res::str::navigate_links()),))
        .id("link-list"),
        spacer(),
    ))
    .spacing(12.0)
    .align(HAlign::Leading)
    .grow()
    .padding(16.0)
}
