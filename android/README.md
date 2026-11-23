# Charlotte Browser — Android

This folder contains a **real, installable Android WebView browser app**, not
the desktop Rust binary repackaged as an APK. It includes a native address bar,
HTTPS navigation, back/forward/reload, a packaged offline start page, downloads,
and settings for toolbar auto-hide and browsing-data clearing.

The **Rust desktop AI automation engine, password vault, extension APIs, and
desktop CEF runtime are NOT embedded in Android**. Android System WebView is
the renderer and controls the browser sandbox and website storage.

## Build a preview APK (requires Android SDK 35, JDK 17, Gradle 8.9)

```bash
cd android
gradle assembleDebug
```

The output `app/build/outputs/apk/debug/app-debug.apk` is **debug-signed**
and installable on Android 8+ (API 26 or later). The GitHub workflow
`Charlotte Android APK` builds and verifies that package and publishes it
as a downloadable workflow artifact, `charlotte-android-preview.apk`.

### Install and launch on an attached Android device

Enable developer options and USB debugging, then:

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -W -n com.noethric.charlotte.debug/com.noethric.charlotte.MainActivity
```

Alternatively transfer the preview APK to Android, tap it and confirm
installation in the standard Android package installer. The OS may ask you to
allow installation from that source. Open **Charlotte Browser** from your
launcher. Never bypass Android's installer confirmation or permission model.

## Production release APK

1. Create a secure Android app-signing keystore and retain it: existing Android
   installation updates require the same signing identity.
2. In the GitHub repo secrets configure `ANDROID_KEYSTORE_BASE64`,
   `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, and
   `ANDROID_KEY_PASSWORD`.
3. Push a tag `android-v0.4.0` after Android CI is green. The workflow creates
   and verifies `Charlotte-Android.apk` using the release signing key,
   then attaches it to the GitHub release.

**Do not publish an unsigned `assembleRelease` APK:** it cannot be installed
as a normal app. The preview artifact uses Android's development signing key,
and is **not** a replacement for production release signing.

## APK updates

The Android menu provides a link to the project's official releases. An APK
download is handled through Android DownloadManager and installed only when
the user opens it and approves the system package installer. Silent installation
of arbitrary APKs is intentionally unsupported.

## Security notes

- Only HTTPS addresses are accepted via the address bar.
- File/content schemes, JavaScript URLs, mixed content and third-party WebView
  app file access are disabled.
- JavaScript and DOM storage remain enabled because real websites need them.
- Website downloads are HTTPS-only and saved to Android Downloads.
- This project does **not** promise Chromium desktop extension compatibility.
