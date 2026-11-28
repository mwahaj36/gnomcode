package dev.gnomcode.app;

import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    private static final String AUTH_PAGE = "https://gnomcode.vercel.app/auth";

    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(WidgetBridgePlugin.class);
        super.onCreate(savedInstanceState);
        keepClearOfSystemBars();
        handleAuthReturn(getIntent());
    }

    /** Android 15+ draws edge to edge; keep the page out from under the status and navigation bars. */
    private void keepClearOfSystemBars() {
        if (bridge == null || bridge.getWebView() == null) return;
        View web = bridge.getWebView();
        ViewCompat.setOnApplyWindowInsetsListener(web, (view, insets) -> {
            Insets bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() | WindowInsetsCompat.Type.displayCutout() | WindowInsetsCompat.Type.ime());
            ViewGroup.MarginLayoutParams params = (ViewGroup.MarginLayoutParams) view.getLayoutParams();
            params.setMargins(bars.left, bars.top, bars.right, bars.bottom);
            view.setLayoutParams(params);
            return WindowInsetsCompat.CONSUMED;
        });
        ViewCompat.requestApplyInsets(web);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        handleAuthReturn(intent);
    }

    /** The provider returns to dev.gnomcode.app://auth/callback?code=...; the web page exchanges the code (PKCE). */
    private void handleAuthReturn(Intent intent) {
        Uri data = intent == null ? null : intent.getData();
        if (data == null || !"dev.gnomcode.app".equals(data.getScheme()) || !"auth".equals(data.getHost())) return;
        String code = data.getQueryParameter("code");
        String error = data.getQueryParameter("error_description");
        Uri.Builder target = Uri.parse(AUTH_PAGE).buildUpon();
        if (code != null && code.length() <= 512) target.appendQueryParameter("code", code);
        else target.appendQueryParameter("error_description", error == null ? "Sign-in was cancelled." : error);
        if (bridge != null && bridge.getWebView() != null) bridge.getWebView().loadUrl(target.build().toString());
    }
}
