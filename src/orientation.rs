// SPDX-License-Identifier: GPL-2.0-or-later
//! Native rotation policy, scoped to the open game. Catalogs and adaptive games stay unlocked.
pub fn portrait(locked: bool) {
    #[cfg(target_os = "ios")]
    unsafe {
        unsafe extern "C" {
            fn games_fair_set_portrait(locked: bool);
        }
        games_fair_set_portrait(locked);
    }
    #[cfg(target_os = "android")]
    day_platform_android::java(|env| GamesFairOrientation::setPortrait(env, locked));
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let _ = locked;
}
#[cfg(target_os = "android")]
day_platform_android::java_class! {
    GamesFairOrientation => dev.daybrite.day.bridge.GamesFairOrientation,
    methods { static fn setPortrait(locked: bool), },
}
