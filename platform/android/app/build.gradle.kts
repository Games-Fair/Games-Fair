// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
plugins {
    // Configures this module from Day.toml and the app's pieces
    // (https://daybrite.dev/docs/platforms/android-mdc#the-gradle-project).
    id("dev.daybrite.day.android")
}

// This app's own Android settings and libraries. They apply after Day's plugin, so they win.
android {
}

dependencies {
}
