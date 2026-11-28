package dev.gnomcode.app;

import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.Context;
import android.widget.RemoteViews;

import org.json.JSONArray;
import org.json.JSONObject;

/** Agents that need the user; tapping opens Remote. */
public class WaitingWidgetProvider extends AppWidgetProvider {
    @Override
    public void onUpdate(Context context, AppWidgetManager mgr, int[] ids) {
        updateAll(context, mgr, ids);
    }

    static void updateAll(Context context, AppWidgetManager mgr, int[] ids) {
        if (ids == null) return;
        JSONObject snap = WidgetData.load(context);
        JSONArray waiting = snap == null ? null : snap.optJSONArray("waiting");
        int n = waiting == null ? 0 : waiting.length();
        String headline;
        String detail;
        if (snap == null) {
            headline = "Not synced";
            detail = "Sign in to Gnomcode Remote";
        } else if (n == 0) {
            headline = "All agents idle";
            detail = "Nothing needs you";
        } else {
            JSONObject first = waiting.optJSONObject(0);
            headline = n == 1 ? "1 agent needs you" : n + " agents need you";
            detail = first == null ? "" : WidgetData.providerName(first.optString("agent")) + " · " + first.optString("project");
        }
        for (int id : ids) {
            RemoteViews v = new RemoteViews(context.getPackageName(), R.layout.widget_waiting);
            v.setTextViewText(R.id.headline, headline);
            v.setTextViewText(R.id.detail, detail);
            v.setTextViewText(R.id.updated, WidgetData.updatedLabel(snap));
            v.setOnClickPendingIntent(R.id.root, WidgetData.openApp(context));
            mgr.updateAppWidget(id, v);
        }
    }
}
