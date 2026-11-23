package com.noethric.charlotte;

import java.net.URI;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;

/** Restrict address-bar navigation to HTTPS and safe search queries. */
public final class Navigation {
    private Navigation() {}

    public static String resolve(String input) {
        if (input == null) return null;
        String text = input.trim();
        if (text.isEmpty() || text.length() > 2048) return null;
        String lower = text.toLowerCase(java.util.Locale.ROOT);
        if (lower.startsWith("javascript:") || lower.startsWith("file:")
                || lower.startsWith("data:") || lower.startsWith("content:")
                || lower.startsWith("intent:") || lower.startsWith("about:"))
            return null;
        if (lower.startsWith("https://")) return safeHttps(text) ? text : null;
        if (text.contains("://")) return null;
        if (!text.contains(" ") && text.contains(".")) {
            String url = "https://" + text;
            if (safeHttps(url)) return url;
        }
        return "https://www.google.com/search?q=" +
                URLEncoder.encode(text, StandardCharsets.UTF_8);
    }

    public static boolean safeHttps(String input) {
        try {
            URI uri = new URI(input);
            return "https".equalsIgnoreCase(uri.getScheme())
                    && uri.getHost() != null
                    && uri.getUserInfo() == null;
        } catch (Exception ignored) {
            return false;
        }
    }
}
