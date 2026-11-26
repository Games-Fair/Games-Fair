// SPDX-License-Identifier: GPL-2.0-or-later
//! Generated, typed access to the app's shared translations and bundled assets.
//! Game crates depend on this leaf crate, never on the app shell.
pub mod res {
    include!(concat!(env!("OUT_DIR"), "/day_resources.rs"));
}
