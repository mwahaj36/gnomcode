package dev.gnomcode.app;

import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.Context;
import android.widget.RemoteViews;

import org.json.JSONObject;

/** Activity heatmap with total tokens and current streak. */
public class ActivityWidgetProvider extends AppWidgetProvider {
    static final int WEEKS = 17;

    @Override
    public void onUpdate(Context context, AppWidgetManager mgr, int[] ids) {
        updateAll(context, mgr, ids);
    }

    static void updateAll(Context context, AppWidgetManager mgr, int[] ids) {
        if (ids == null) return;
        JSONObject snap = WidgetData.load(context);
        JSONObject heat = snap == null ? null : snap.optJSONObject("heatmap");
        JSONObject stats = heat == null ? null : heat.optJSONObject("stats");
        String line;
        if (stats != null) {
            line = WidgetData.compact(stats.optLong("total_tokens")) + " tokens · " + stats.optInt("current_streak") + " day streak";
        } else {
            line = snap == null ? "Sign in to Gnomcode Remote" : "Turn on Account sync on your desktop";
        }
        float density = context.getResources().getDisplayMetrics().density;
        for (int id : ids) {
            RemoteViews v = new RemoteViews(context.getPackageName(), R.layout.widget_activity);
            v.setImageViewBitmap(R.id.heatmap, WidgetData.heatmap(heat == null ? null : heat.optJSONArray("days"), WEEKS, density));
            v.setTextViewText(R.id.stats, line);
            v.setTextViewText(R.id.updated, WidgetData.updatedLabel(snap));
            v.setOnClickPendingIntent(R.id.root, WidgetData.openApp(context));
            mgr.updateAppWidget(id, v);
        }
    }
}
