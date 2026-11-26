package app.streamphone;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import com.google.zxing.integration.android.IntentIntegrator;
import com.google.zxing.integration.android.IntentResult;

import java.net.HttpURLConnection;
import java.net.InetAddress;
import java.net.URL;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * The phone side of StreamPhone: a full-screen window onto the dashboard the PC
 * serves. It looks for the PC over the USB cable first (the PC forwards its
 * port, so it appears as localhost), then at the Wi-Fi address it was paired
 * with. Compared with the browser it keeps the screen on, hides the system
 * bars, and turns pages with the volume keys.
 */
public class MainActivity extends Activity {

    /** What the PC's USB port forward looks like from the phone. */
    private static final String USB = "http://localhost:47800/";
    private static final String PREFS = "streamphone";
    private static final String KEY_WIFI = "wifi";
    private static final int PORT = 47800;

    private final Handler main = new Handler(Looper.getMainLooper());
    private WebView web;
    private LinearLayout setup;
    private TextView status;
    private Button forget;
    private SharedPreferences prefs;

    /** The address the dashboard was loaded from; null while looking for the PC. */
    private String connected;
    /** Bumped whenever a search starts, so an older one's answer is ignored. */
    private int search;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        prefs = getSharedPreferences(PREFS, MODE_PRIVATE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(Color.BLACK);
        web = createWebView();
        root.addView(web, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        setup = createSetup();
        root.addView(setup, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        setContentView(root);

        if (android.os.Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::back);
        }

        findComputer();
    }

    @Override
    protected void onResume() {
        super.onResume();
        hideSystemBars();
        web.onResume();
        // Plugged in or back in range while the app was in the background.
        if (connected == null) findComputer();
    }

    @Override
    protected void onPause() {
        web.onPause();
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        main.removeCallbacksAndMessages(null);
        web.destroy();
        super.onDestroy();
    }

    @Override
    public void onWindowFocusChanged(boolean focused) {
        super.onWindowFocusChanged(focused);
        if (focused) hideSystemBars();
    }

    private void hideSystemBars() {
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController bars = getWindow().getInsetsController();
            if (bars != null) {
                bars.hide(WindowInsets.Type.systemBars());
                bars.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            }
        } else {
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY | View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
        }
    }

    // ---- the dashboard --------------------------------------------------------

    @SuppressLint("SetJavaScriptEnabled")
    private WebView createWebView() {
        WebView view = new WebView(this);
        view.setBackgroundColor(Color.BLACK);
        view.setOverScrollMode(View.OVER_SCROLL_NEVER);
        WebSettings settings = view.getSettings();
        settings.setJavaScriptEnabled(true);
        // The dashboard keeps its Wi-Fi pairing in localStorage.
        settings.setDomStorageEnabled(true);
        settings.setMediaPlaybackRequiresUserGesture(false);
        settings.setTextZoom(100);
        settings.setSupportZoom(false);
        // Web content only: no files or content providers of the phone.
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);

        view.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest request) {
                // The dashboard never leaves the PC it was loaded from.
                return connected == null || !request.getUrl().toString().startsWith(connected);
            }

            @Override
            public void onReceivedError(WebView v, WebResourceRequest request, WebResourceError error) {
                if (request.isForMainFrame()) lost();
            }
        });
        return view;
    }

    /** The page could not be loaded: the cable was pulled or the PC went away. */
    private void lost() {
        connected = null;
        web.loadUrl("about:blank");
        findComputer();
    }

    private void show(String base, String url) {
        connected = base;
        setup.setVisibility(View.GONE);
        web.loadUrl(url);
    }

    // ---- finding the PC -------------------------------------------------------

    /** Tries the cable, then the paired Wi-Fi address; keeps trying until one answers. */
    private void findComputer() {
        final int mine = ++search;
        final List<String> candidates = new ArrayList<>();
        candidates.add(USB);
        String wifi = prefs.getString(KEY_WIFI, null);
        if (wifi != null) candidates.add(wifi);

        setup.setVisibility(View.VISIBLE);
        forget.setVisibility(wifi != null ? View.VISIBLE : View.GONE);
        status.setText(R.string.searching);

        new Thread(() -> {
            String found = null;
            for (String candidate : candidates) {
                if (answers(candidate)) {
                    found = candidate;
                    break;
                }
            }
            final String base = found;
            main.post(() -> {
                if (mine != search || isFinishing()) return;
                if (base != null) {
                    show(base, base);
                } else {
                    status.setText(R.string.not_found);
                    main.postDelayed(() -> {
                        if (mine == search && connected == null) findComputer();
                    }, 3000);
                }
            });
        }).start();
    }

    /** Whether a StreamPhone agent serves its dashboard at this address. */
    private static boolean answers(String base) {
        HttpURLConnection http = null;
        try {
            http = (HttpURLConnection) new URL(base + "manifest.webmanifest").openConnection();
            http.setConnectTimeout(1500);
            http.setReadTimeout(1500);
            http.setUseCaches(false);
            return http.getResponseCode() == 200;
        } catch (Exception unreachable) {
            return false;
        } finally {
            if (http != null) http.disconnect();
        }
    }

    // ---- pairing over Wi-Fi ---------------------------------------------------

    /**
     * Accepts the link from the PC's QR code, {@code http://192.168.0.10:47800/#pair=CODE}:
     * remembers the address and lets the dashboard trade the code for a pairing.
     */
    private boolean pair(String link) {
        Uri uri = link == null ? null : Uri.parse(link.trim());
        if (uri == null || !"http".equals(uri.getScheme()) || uri.getHost() == null || !isLocal(uri.getHost())) return false;
        String fragment = uri.getFragment();
        if (fragment == null || !fragment.matches("pair=[A-Za-z0-9]{4,16}")) return false;

        int port = uri.getPort() > 0 ? uri.getPort() : PORT;
        String base = "http://" + uri.getHost() + ":" + port + "/";
        prefs.edit().putString(KEY_WIFI, base).apply();
        search++;
        show(base, base + "#" + fragment);
        return true;
    }

    /** Only addresses on the local network: the app never opens the dashboard of some site. */
    private static boolean isLocal(String host) {
        if (!host.matches("\\d{1,3}(\\.\\d{1,3}){3}")) return false;
        try {
            InetAddress address = InetAddress.getByName(host);
            return address.isSiteLocalAddress() || address.isLoopbackAddress();
        } catch (Exception invalid) {
            return false;
        }
    }

    private void scan() {
        IntentIntegrator scanner = new IntentIntegrator(this);
        scanner.setDesiredBarcodeFormats(IntentIntegrator.QR_CODE);
        scanner.setPrompt(getString(R.string.scan_prompt));
        scanner.setBeepEnabled(false);
        scanner.setOrientationLocked(true);
        scanner.initiateScan();
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        IntentResult scanned = IntentIntegrator.parseActivityResult(request, result, data);
        if (scanned == null) {
            super.onActivityResult(request, result, data);
        } else if (scanned.getContents() != null && !pair(scanned.getContents())) {
            Toast.makeText(this, R.string.bad_link, Toast.LENGTH_LONG).show();
        }
    }

    /** For phones without a camera: the address and the code, typed in. */
    private void typeAddress() {
        final EditText address = new EditText(this);
        address.setHint(R.string.address_hint);
        address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        address.setSingleLine();
        final EditText code = new EditText(this);
        code.setHint(R.string.code_hint);
        code.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        code.setSingleLine();

        LinearLayout form = new LinearLayout(this);
        form.setOrientation(LinearLayout.VERTICAL);
        int pad = dp(20);
        form.setPadding(pad, dp(8), pad, 0);
        form.addView(address);
        form.addView(code);

        new AlertDialog.Builder(this)
                .setTitle(R.string.address_title)
                .setView(form)
                .setNegativeButton(R.string.cancel, null)
                .setPositiveButton(R.string.connect, (dialog, which) -> {
                    String host = address.getText().toString().trim().replaceFirst("^https?://", "").replaceFirst("/.*$", "");
                    if (!host.contains(":")) host += ":" + PORT;
                    String typed = code.getText().toString().trim().toUpperCase(Locale.ROOT);
                    if (!pair("http://" + host + "/#pair=" + typed)) {
                        Toast.makeText(this, R.string.bad_address, Toast.LENGTH_LONG).show();
                    }
                })
                .show();
    }

    // ---- the setup screen -----------------------------------------------------

    private LinearLayout createSetup() {
        LinearLayout box = new LinearLayout(this);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setGravity(Gravity.CENTER);
        box.setBackgroundColor(Color.BLACK);
        box.setPadding(dp(32), dp(16), dp(32), dp(16));
        // Taps must not fall through to the dashboard underneath.
        box.setClickable(true);

        TextView title = text(R.string.app_name, 30, 0xFFF5F5F7);
        title.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        title.setGravity(Gravity.CENTER);
        box.addView(title);

        status = text(R.string.searching, 16, 0xFFFF9F0A);
        status.setPadding(0, dp(6), 0, 0);
        status.setGravity(Gravity.CENTER);
        box.addView(status);

        TextView hint = text(R.string.hint, 14, 0xFFA1A1A8);
        hint.setGravity(Gravity.CENTER);
        hint.setPadding(0, dp(12), 0, dp(18));
        hint.setMaxWidth(dp(520));
        box.addView(hint);

        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER);
        row.addView(button(R.string.scan, 0xFF0A84FF, v -> scan()));
        row.addView(button(R.string.type, 0xFF2C2C2E, v -> typeAddress()));
        row.addView(button(R.string.retry, 0xFF2C2C2E, v -> findComputer()));
        forget = button(R.string.forget, 0xFF2C2C2E, v -> {
            prefs.edit().remove(KEY_WIFI).apply();
            findComputer();
        });
        row.addView(forget);
        box.addView(row);
        return box;
    }

    private TextView text(int resource, int sp, int color) {
        TextView view = new TextView(this);
        view.setText(resource);
        view.setTextSize(TypedValue.COMPLEX_UNIT_SP, sp);
        view.setTextColor(color);
        return view;
    }

    private Button button(int label, int color, View.OnClickListener action) {
        Button view = new Button(this);
        view.setText(label);
        view.setAllCaps(false);
        view.setTextColor(Color.WHITE);
        view.setTextSize(TypedValue.COMPLEX_UNIT_SP, 15);
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(color);
        shape.setCornerRadius(dp(12));
        view.setBackground(shape);
        view.setPadding(dp(18), dp(10), dp(18), dp(10));
        view.setStateListAnimator(null);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.setMargins(dp(5), 0, dp(5), 0);
        view.setLayoutParams(params);
        view.setOnClickListener(action);
        return view;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    // ---- keys -----------------------------------------------------------------

    /** The volume keys turn the dashboard's pages, like a stream deck's page buttons. */
    @Override
    public boolean onKeyDown(int code, KeyEvent event) {
        if (connected != null && (code == KeyEvent.KEYCODE_VOLUME_UP || code == KeyEvent.KEYCODE_VOLUME_DOWN)) {
            String key = code == KeyEvent.KEYCODE_VOLUME_UP ? "ArrowRight" : "ArrowLeft";
            web.evaluateJavascript("window.dispatchEvent(new KeyboardEvent('keydown',{key:'" + key + "',cancelable:true}))", null);
            return true;
        }
        return super.onKeyDown(code, event);
    }

    @Override
    public boolean onKeyUp(int code, KeyEvent event) {
        // Swallow the matching key-up, or the system shows its volume panel.
        if (connected != null && (code == KeyEvent.KEYCODE_VOLUME_UP || code == KeyEvent.KEYCODE_VOLUME_DOWN)) return true;
        return super.onKeyUp(code, event);
    }

    /** Back opens the connection screen instead of closing the dashboard by accident. */
    private void back() {
        if (connected != null) {
            String wifi = prefs.getString(KEY_WIFI, null);
            status.setText(connected.equals(USB) ? getString(R.string.over_usb) : getString(R.string.over_wifi, Uri.parse(connected).getHost()));
            forget.setVisibility(wifi != null ? View.VISIBLE : View.GONE);
            setup.setVisibility(setup.getVisibility() == View.VISIBLE ? View.GONE : View.VISIBLE);
        } else {
            finish();
        }
    }

    /** Android 12 and older deliver Back here; newer ones through the callback registered in onCreate. */
    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        back();
    }
}
