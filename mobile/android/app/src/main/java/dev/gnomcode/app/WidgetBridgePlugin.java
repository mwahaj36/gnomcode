package dev.gnomcode.app;

import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

/** Receives the content-free widget snapshot from the Remote page (ADR 0066). */
@CapacitorPlugin(name = "WidgetBridge")
public class WidgetBridgePlugin extends Plugin {
    @PluginMethod
    public void setSnapshot(PluginCall call) {
        String json = call.getString("json");
        if (json == null || json.length() > 200_000) {
            call.reject("invalid snapshot");
            return;
        }
        WidgetData.save(getContext(), json);
        WidgetData.refreshAll(getContext());
        call.resolve();
    }
}
