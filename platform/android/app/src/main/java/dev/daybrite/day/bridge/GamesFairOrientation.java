// SPDX-License-Identifier: GPL-2.0-or-later
package dev.daybrite.day.bridge;

import android.app.Activity;
import android.content.pm.ActivityInfo;

/** The app's portrait boards must not be compressed by a landscape viewport. */
public final class GamesFairOrientation {
    public static void setPortrait(boolean locked) {
        Activity activity = DayBridge.primaryActivity.get();
        if (activity == null) return;
        activity.runOnUiThread(() -> activity.setRequestedOrientation(locked
            ? ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
            : ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED));
    }
}
