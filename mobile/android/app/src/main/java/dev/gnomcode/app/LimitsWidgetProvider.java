package dev.gnomcode.app;

import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.Context;
import android.view.View;
import android.widget.RemoteViews;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.Locale;

/** One row per provider: the headline window's remaining percent and reset time. */
public class LimitsWidgetProvider extends AppWidgetProvider {
    private static final int[][] ROWS = {
        {R.id.row0, R.id.name0, R.id.bar0, R.id.detail0},
        {R.id.row1, R.id.name1, R.id.bar1, R.id.detail1},
        {R.id.row2, R.id.name2, R.id.bar2, R.id.detail2},
    };

    @Override
    public void onUpdate(Context context, AppWidgetManager mgr, int[] ids) {
        updateAll(context, mgr, ids);
    }

    static void updateAll(Context context, AppWidgetManager mgr, int[] ids) {
        if (ids == null) return;
        JSONObject snap = WidgetData.load(context);
        JSONArray limits = snap == null ? null : snap.optJSONArray("limits");
        int count = limits == null ? 0 : Math.min(limits.length(), ROWS.length);
        for (int id : ids) {
            RemoteViews v = new RemoteViews(context.getPackageName(), R.layout.widget_limits);
            for (int i = 0; i < ROWS.length; i++) {
                int[] row = ROWS[i];
                JSONObject item = i < count ? limits.optJSONObject(i) : null;
                if (item == null) {
                    v.setViewVisibility(row[0], View.GONE);
                    continue;
                }
                int left = (int) Math.round(Math.max(0, Math.min(100, 100 - item.optDouble("used_pct", 0))));
                String reset = WidgetData.resetLabel(item.optString("resets_at", ""));
                boolean exact = "exact".equals(item.optString("basis"));
                v.setViewVisibility(row[0], View.VISIBLE);
                v.setTextViewText(row[1], WidgetData.providerName(item.optString("provider")));
                v.setProgressBar(row[2], 100, left, false);
                v.setTextViewText(row[3], String.format(Locale.US, "%d%% left%s%s", left, reset.isEmpty() ? "" : " · " + reset, exact ? "" : " · EST"));
            }
            v.setViewVisibility(R.id.empty, count == 0 ? View.VISIBLE : View.GONE);
            v.setTextViewText(R.id.empty, snap == null ? "Sign in to Gnomcode Remote" : "No limits in the last snapshot");
            v.setTextViewText(R.id.updated, WidgetData.updatedLabel(snap));
            v.setOnClickPendingIntent(R.id.root, WidgetData.openApp(context));
            mgr.updateAppWidget(id, v);
        }
    }
}
