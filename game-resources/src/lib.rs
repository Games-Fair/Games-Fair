// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Generated, typed access to the app's shared translations and bundled assets.
//! Game crates depend on this leaf crate, never on the app shell.
pub mod res {
    include!(concat!(env!("OUT_DIR"), "/day_resources.rs"));
}
