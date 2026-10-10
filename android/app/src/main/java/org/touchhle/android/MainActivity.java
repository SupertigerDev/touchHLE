/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Parts of this file are derived from SDL 2's Android project template, which
 * has a different license. Please see vendor/SDL/LICENSE.txt for details.
 */
package org.touchhle.android;

import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import org.libsdl.app.SDLActivity;

/**
 * A wrapper class over SDLActivity
 */

public class MainActivity extends SDLActivity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Intent service = new Intent(this, KeepAliveService.class);
        if (Build.VERSION.SDK_INT >= 26) {
            startForegroundService(service);
        } else {
            startService(service);
        }
        if (Build.VERSION.SDK_INT >= 33
            && checkSelfPermission("android.permission.POST_NOTIFICATIONS")
                != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {"android.permission.POST_NOTIFICATIONS"}, 1);
        }
    }

    @Override
    protected void onDestroy() {
        stopService(new Intent(this, KeepAliveService.class));
        super.onDestroy();
    }

    @Override
    protected void onUserLeaveHint() {
        super.onUserLeaveHint();
        if (SDLActivity.nativeGetHintBoolean("TOUCHHLE_IPA_RUNNING", false)) {
            finish();
        }
    }

    @Override
    protected String[] getLibraries() {
        return new String[]{
            "SDL2",
            "touchHLE"
        };
    }
}
