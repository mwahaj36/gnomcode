package dev.gnomcode.app;

import android.app.PendingIntent;
import android.appwidget.AppWidgetManager;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;

import org.json.JSONArray;
import org.json.JSONObject;

import java.time.Instant;
import java.time.LocalDate;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.Locale;

/** Stores the widget snapshot and renders shared pieces. Missing or malformed data never crashes a widget. */
final class WidgetData {
    private static final String PREFS = "gnomcode_widgets";
    private static final String KEY = "snapshot";
    static final int[] LEVELS = {0xFF1A1E29, 0xFF26306E, 0xFF3346B0, 0xFF5A6FE0, 0xFF9AA8FF};

    private WidgetData() {}

    static void save(Context context, String json) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY, json).apply();
    }

    static JSONObject load(Context context) {
        String raw = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY, null);
        if (raw == null) return null;
        try {
            return new JSONObject(raw);
        } catch (Exception e) {
            return null;
        }
    }

    static void refreshAll(Context context) {
        AppWidgetManager mgr = AppWidgetManager.getInstance(context);
        ActivityWidgetProvider.updateAll(context, mgr, mgr.getAppWidgetIds(new ComponentName(context, ActivityWidgetProvider.class)));
        LimitsWidgetProvider.updateAll(context, mgr, mgr.getAppWidgetIds(new ComponentName(context, LimitsWidgetProvider.class)));
        WaitingWidgetProvider.updateAll(context, mgr, mgr.getAppWidgetIds(new ComponentName(context, WaitingWidgetProvider.class)));
    }

    static PendingIntent openApp(Context context) {
        Intent intent = new Intent(context, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        return PendingIntent.getActivity(context, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    /** Age of the desktop's snapshot, e.g. "Updated 5m ago". */
    static String updatedLabel(JSONObject snap) {
        if (snap == null) return "Open Gnomcode to sync";
        try {
            long ageMin = Math.max(0, (System.currentTimeMillis() - Instant.parse(snap.getString("generated_at")).toEpochMilli()) / 60_000);
            if (ageMin < 1) return "Updated just now";
            if (ageMin < 60) return "Updated " + ageMin + "m ago";
            if (ageMin < 48 * 60) return "Updated " + (ageMin / 60) + "h ago";
            return "Updated " + (ageMin / 1440) + "d ago";
        } catch (Exception e) {
            return "Update time unknown";
        }
    }

    static String compact(long n) {
        if (n >= 1_000_000_000L) return String.format(Locale.US, "%.1fB", n / 1e9);
        if (n >= 1_000_000L) return String.format(Locale.US, "%.1fM", n / 1e6);
        if (n >= 1_000L) return String.format(Locale.US, "%.1fK", n / 1e3);
        return String.valueOf(n);
    }

    static String resetLabel(String iso) {
        if (iso == null || iso.isEmpty() || "null".equals(iso)) return "";
        try {
            return "resets " + DateTimeFormatter.ofPattern("EEE HH:mm", Locale.getDefault()).withZone(ZoneId.systemDefault()).format(Instant.parse(iso));
        } catch (Exception e) {
            return "";
        }
    }

    static String providerName(String slug) {
        switch (slug) {
            case "claude-code": return "Claude Code";
            case "codex": return "Codex";
            case "antigravity": return "Antigravity";
            default: return slug;
        }
    }

    /** Heatmap grid, one column per week, Monday on top. Days arrive oldest first with a 0-4 level. */
    static Bitmap heatmap(JSONArray days, int weeks, float density) {
        int cell = Math.round(9 * density);
        int gap = Math.round(2 * density);
        Bitmap bmp = Bitmap.createBitmap(weeks * (cell + gap), 7 * (cell + gap), Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bmp);
        Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        float r = 2 * density;
        int n = days == null ? 0 : days.length();
        int lastRow = LocalDate.now().getDayOfWeek().getValue() - 1;
        if (n > 0) {
            try {
                lastRow = LocalDate.parse(days.getJSONObject(n - 1).getString("date")).getDayOfWeek().getValue() - 1;
            } catch (Exception ignored) {
                // keep today's weekday
            }
        }
        int filled = weeks * 7 - (6 - lastRow);
        int offset = filled - n;
        for (int slot = 0; slot < filled; slot++) {
            int idx = slot - offset;
            int level = 0;
            if (idx >= 0 && idx < n) {
                JSONObject day = days.optJSONObject(idx);
                level = day == null ? 0 : Math.max(0, Math.min(4, day.optInt("level", 0)));
            }
            paint.setColor(LEVELS[level]);
            float x = (slot / 7) * (cell + gap);
            float y = (slot % 7) * (cell + gap);
            canvas.drawRoundRect(new RectF(x, y, x + cell, y + cell), r, r, paint);
        }
        return bmp;
    }
}
