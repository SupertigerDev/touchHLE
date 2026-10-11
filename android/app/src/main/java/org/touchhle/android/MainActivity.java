/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Parts of this file are derived from SDL 2's Android project template, which
 * has a different license. Please see vendor/SDL/LICENSE.txt for details.
 */
package org.touchhle.android;

import android.content.ContentResolver;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.util.Log;
import android.widget.Toast;
import org.libsdl.app.SDLActivity;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;

/**
 * A wrapper class over SDLActivity
 */

public class MainActivity extends SDLActivity {
    private static final int REQUEST_PICK_IPA = 0x484c;
    private static final String TAG = "touchHLE";
    private boolean pickingIpa;

    private static native void nativeOnIpaImportCompleted();

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
        handlePickerIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        handlePickerIntent(intent);
    }

    @Override
    protected void onDestroy() {
        stopService(new Intent(this, KeepAliveService.class));
        super.onDestroy();
    }

    @Override
    protected void onUserLeaveHint() {
        super.onUserLeaveHint();
        if (!pickingIpa && SDLActivity.nativeGetHintBoolean("TOUCHHLE_IPA_RUNNING", false)) {
            finish();
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQUEST_PICK_IPA) {
            return;
        }

        pickingIpa = false;
        if (resultCode != RESULT_OK || data == null || data.getData() == null) {
            return;
        }

        try {
            importIpa(data.getData());
            nativeOnIpaImportCompleted();
        } catch (IOException | SecurityException e) {
            Log.e(TAG, "Couldn't import selected IPA", e);
            Toast.makeText(this, "Couldn't import IPA: " + e.getMessage(), Toast.LENGTH_LONG).show();
        }
    }

    private void handlePickerIntent(Intent intent) {
        Uri data = intent == null ? null : intent.getData();
        if (data == null || !"touchhle".equals(data.getScheme())
            || !"pick-ipa".equals(data.getHost()) || pickingIpa) {
            return;
        }

        Intent picker = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        picker.addCategory(Intent.CATEGORY_OPENABLE);
        picker.setType("*/*");
        pickingIpa = true;
        try {
            startActivityForResult(picker, REQUEST_PICK_IPA);
        } catch (android.content.ActivityNotFoundException e) {
            pickingIpa = false;
            Log.e(TAG, "No document picker is available", e);
            Toast.makeText(this, "No file picker is available", Toast.LENGTH_LONG).show();
        }
    }

    private void importIpa(Uri uri) throws IOException {
        String displayName = null;
        try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (nameIndex >= 0) {
                    displayName = cursor.getString(nameIndex);
                }
            }
        }

        if (displayName == null || !displayName.toLowerCase(java.util.Locale.ROOT).endsWith(".ipa")) {
            throw new IOException("Please select a file with the .ipa extension");
        }

        File filesDirectory = getExternalFilesDir(null);
        if (filesDirectory == null) {
            throw new IOException("App file storage is unavailable");
        }
        File appsDirectory = new File(filesDirectory, "touchHLE_apps");
        if (!appsDirectory.isDirectory() && !appsDirectory.mkdirs()) {
            throw new IOException("Couldn't create " + appsDirectory.getAbsolutePath());
        }

        String safeName = new File(displayName).getName();
        int extensionIndex = safeName.lastIndexOf('.');
        String baseName = safeName.substring(0, extensionIndex);
        String extension = safeName.substring(extensionIndex);
        File imported;
        for (int suffix = 0; ; suffix++) {
            String numberedSuffix = suffix == 0 ? "" : " (" + suffix + ")";
            imported = new File(
                appsDirectory,
                "touchHLE_import_" + baseName + numberedSuffix + extension
            );
            if (imported.createNewFile()) {
                break;
            }
            if (!imported.exists()) {
                throw new IOException("Couldn't create " + imported.getAbsolutePath());
            }
        }

        boolean copied = false;
        try {
            ContentResolver resolver = getContentResolver();
            try (InputStream input = resolver.openInputStream(uri);
                 FileOutputStream output = new FileOutputStream(imported)) {
                if (input == null) {
                    throw new IOException("The selected file couldn't be opened");
                }
                byte[] buffer = new byte[64 * 1024];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    output.write(buffer, 0, count);
                }
            }
            copied = true;
            Toast.makeText(this, "Imported " + displayName, Toast.LENGTH_SHORT).show();
        } finally {
            if (!copied && !imported.delete()) {
                Log.w(TAG, "Couldn't remove incomplete import " + imported.getAbsolutePath());
            }
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
