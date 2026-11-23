package com.noethric.charlotte;

import android.app.Activity;
import android.app.AlertDialog;
import android.app.DownloadManager;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.net.Uri;
import android.os.Bundle;
import android.os.Environment;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.EditorInfo;
import android.webkit.DownloadListener;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.webkit.URLUtil;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.PopupMenu;
import android.widget.ProgressBar;
import android.widget.Toast;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;

/** Installable Android browser shell using Android's real system WebView. */
public final class MainActivity extends Activity {
    private WebView browser;
    private EditText address;
    private LinearLayout toolbar;
    private ProgressBar progress;
    private boolean autoHideToolbar;

    private Button button(String label, View.OnClickListener callback) {
        Button button = new Button(this);
        button.setText(label);
        button.setAllCaps(false);
        button.setTextSize(13);
        button.setOnClickListener(callback);
        return button;
    }

    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        autoHideToolbar = getPreferences(MODE_PRIVATE)
                .getBoolean("auto_hide_toolbar", false);
        getWindow().setStatusBarColor(Color.rgb(17, 24, 39));
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(Color.WHITE);

        toolbar = new LinearLayout(this);
        toolbar.setOrientation(LinearLayout.HORIZONTAL);
        toolbar.setGravity(Gravity.CENTER_VERTICAL);
        toolbar.setBackgroundColor(Color.rgb(239, 242, 247));
        toolbar.addView(button("‹", v -> { if (browser.canGoBack()) browser.goBack(); }));
        toolbar.addView(button("›", v -> { if (browser.canGoForward()) browser.goForward(); }));
        address = new EditText(this);
        address.setSingleLine(true);
        address.setHint("Search or enter website");
        address.setTextSize(14);
        address.setImeOptions(EditorInfo.IME_ACTION_GO);
        toolbar.addView(address, new LinearLayout.LayoutParams(
                0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        address.setOnEditorActionListener((v, action, event) -> {
            if (action == EditorInfo.IME_ACTION_GO
                    || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER)) {
                goTo(address.getText().toString());
                return true;
            }
            return false;
        });
        toolbar.addView(button("↻", v -> browser.reload()));
        Button menu = button("⋮", v -> {});
        toolbar.addView(menu);
        menu.setOnClickListener(v -> showMenu(menu));
        root.addView(toolbar, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        progress = new ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal);
        progress.setMax(100);
        progress.setVisibility(View.GONE);
        root.addView(progress, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 3));

        browser = new WebView(this);
        WebSettings settings = browser.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setAllowFileAccessFromFileURLs(false);
        settings.setAllowUniversalAccessFromFileURLs(false);
        settings.setJavaScriptCanOpenWindowsAutomatically(false);
        settings.setSupportMultipleWindows(false);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
        settings.setSafeBrowsingEnabled(true);
        settings.setMediaPlaybackRequiresUserGesture(true);
        browser.setWebViewClient(new WebViewClient() {
            @Override public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                if (!request.isForMainFrame()) return false;
                String scheme = request.getUrl().getScheme();
                if ("https".equalsIgnoreCase(scheme)) return false;
                toast("Blocked unsupported or insecure navigation");
                return true;
            }
            @Override public void onPageFinished(WebView view, String url) {
                if (Navigation.safeHttps(url) && !url.contains("appassets.androidplatform.net")) {
                    address.setText(url);
                }
            }
        });
        browser.setWebChromeClient(new WebChromeClient() {
            @Override public void onProgressChanged(WebView view, int value) {
                progress.setVisibility(value == 100 ? View.GONE : View.VISIBLE);
                progress.setProgress(value);
            }
        });
        browser.setDownloadListener((url, userAgent, contentDisposition, mimeType, size) -> {
            if (!Navigation.safeHttps(url)) {
                toast("Only HTTPS downloads are supported");
                return;
            }
            try {
                String file = URLUtil.guessFileName(url, contentDisposition, mimeType);
                DownloadManager.Request request = new DownloadManager.Request(Uri.parse(url));
                request.setTitle(file);
                request.setNotificationVisibility(
                        DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED);
                request.setDestinationInExternalPublicDir(Environment.DIRECTORY_DOWNLOADS, file);
                DownloadManager manager =
                        (DownloadManager) getSystemService(Context.DOWNLOAD_SERVICE);
                manager.enqueue(request);
                toast("Downloading to the Android Downloads folder");
            } catch (Exception error) {
                toast("Download failed: " + error.getMessage());
            }
        });
        browser.setOnScrollChangeListener((v, x, y, oldX, oldY) -> {
            if (!autoHideToolbar) return;
            if (y > oldY && y > 50) toolbar.setVisibility(View.GONE);
            else if (y < oldY) toolbar.setVisibility(View.VISIBLE);
        });
        root.addView(browser, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        setContentView(root);

        String launch = getIntent() != null && getIntent().getData() != null
                ? getIntent().getData().toString() : null;
        if (launch != null && Navigation.safeHttps(launch)) browser.loadUrl(launch);
        else showHome();
    }

    private void toast(String message) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show();
    }

    private void goTo(String input) {
        String url = Navigation.resolve(input);
        if (url == null) { toast("Enter a valid HTTPS address or search"); return; }
        browser.loadUrl(url);
        address.clearFocus();
    }

    private void showHome() {
        try (InputStream stream = getAssets().open("start.html")) {
            byte[] bytes = new byte[stream.available()];
            int total = 0;
            while (total < bytes.length) {
                int n = stream.read(bytes, total, bytes.length - total);
                if (n < 0) break;
                total += n;
            }
            String html = new String(bytes, 0, total, StandardCharsets.UTF_8);
            browser.loadDataWithBaseURL("https://appassets.androidplatform.net/",
                    html, "text/html", "UTF-8", null);
            address.setText("");
        } catch (Exception error) {
            browser.loadData("<h1>Charlotte Browser</h1>", "text/html", "UTF-8");
        }
        toolbar.setVisibility(View.VISIBLE);
    }

    private void showMenu(View anchor) {
        PopupMenu menu = new PopupMenu(this, anchor);
        menu.getMenu().add(0, 1, 0, "Home");
        menu.getMenu().add(0, 2, 1, "Personalization");
        menu.getMenu().add(0, 3, 2, "Passwords & Security");
        menu.getMenu().add(0, 4, 3, "AI & Agent");
        menu.getMenu().add(0, 5, 4, "Extensions Marketplace");
        menu.getMenu().add(0, 6, 5, "Check Android releases");
        menu.setOnMenuItemClickListener(item -> {
            switch (item.getItemId()) {
                case 1: showHome(); return true;
                case 2: showPersonalization(); return true;
                case 3: showSecurity(); return true;
                case 4:
                    info("AI & Agent", "Desktop Rust agent integration is not yet available on Android.");
                    return true;
                case 5:
                    info("Extensions Marketplace",
                            "Third-party extension installation is not supported by Android WebView.");
                    return true;
                case 6:
                    browser.loadUrl("https://github.com/EMN90909/Charlotte/releases");
                    return true;
                default: return false;
            }
        });
        menu.show();
    }

    private void showPersonalization() {
        CheckBox check = new CheckBox(this);
        check.setText("Auto-hide address bar when scrolling down");
        check.setChecked(autoHideToolbar);
        int pad = Math.round(20 * getResources().getDisplayMetrics().density);
        check.setPadding(pad, pad, pad, pad);
        new AlertDialog.Builder(this).setTitle("Personalization")
                .setView(check)
                .setPositiveButton("Save", (dialog, which) -> {
                    autoHideToolbar = check.isChecked();
                    getPreferences(MODE_PRIVATE).edit()
                            .putBoolean("auto_hide_toolbar", autoHideToolbar).apply();
                    if (!autoHideToolbar) toolbar.setVisibility(View.VISIBLE);
                }).setNegativeButton("Cancel", null).show();
    }

    private void showSecurity() {
        new AlertDialog.Builder(this).setTitle("Passwords & Security")
                .setMessage("Android's WebView manages website cookies. Charlotte does not yet offer " +
                        "a standalone password vault on Android. Clear browser cache and cookies?")
                .setPositiveButton("Clear browsing data", (d, which) -> {
                    browser.clearCache(true);
                    browser.clearHistory();
                    android.webkit.CookieManager.getInstance().removeAllCookies(null);
                    android.webkit.WebStorage.getInstance().deleteAllData();
                    toast("Browser data cleared");
                }).setNegativeButton("Cancel", null).show();
    }

    private void info(String title, String message) {
        new AlertDialog.Builder(this).setTitle(title).setMessage(message)
                .setPositiveButton("OK", null).show();
    }

    @Override public void onBackPressed() {
        if (browser != null && browser.canGoBack()) browser.goBack();
        else super.onBackPressed();
    }

    @Override protected void onDestroy() {
        if (browser != null) {
            browser.stopLoading();
            browser.destroy();
        }
        super.onDestroy();
    }
}
