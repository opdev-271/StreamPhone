package app.streamphone;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
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
import android.webkit.RenderProcessGoneDetail;
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

import java.io.PrintWriter;
import java.io.StringWriter;
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
    private static final String KEY_CRASH = "crash";
    /** Tells the dashboard it runs inside the app, so it does not offer to open the app. */
    private static final String AGENT_MARK = " StreamPhoneApp/";
    private static final int PORT = 47800;

    private final Handler main = new Handler(Looper.getMainLooper());
    private FrameLayout root;
    private WebView web;
    private LinearLayout setup;
    private TextView status;
    private TextView crashed;
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
        rememberCrashes();
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        root = new FrameLayout(this);
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
        open(getIntent());
    }

    /** The app is already running and a pairing link was opened from a QR reader or the browser. */
    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        open(intent);
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
        settings.setUserAgentString(settings.getUserAgentString() + AGENT_MARK + BuildConfig.VERSION_NAME);

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

            @Override
            public boolean onRenderProcessGone(WebView v, RenderProcessGoneDetail detail) {
                // The system killed the page's process (low memory, a WebView
                // bug). Left unhandled this closes the whole app; start over
                // with a fresh view instead.
                main.post(() -> replaceWebView(v));
                return true;
            }
        });
        return view;
    }

    private void replaceWebView(WebView dead) {
        if (dead != web || isFinishing()) return;
        int at = root.indexOfChild(dead);
        root.removeView(dead);
        dead.destroy();
        web = createWebView();
        root.addView(web, Math.max(at, 0), new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        connected = null;
        findComputer();
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
        if (!valid(uri)) return false;
        String fragment = uri.getFragment();

        int port = uri.getPort() > 0 ? uri.getPort() : PORT;
        String base = "http://" + uri.getHost() + ":" + port + "/";
        prefs.edit().putString(KEY_WIFI, base).apply();
        search++;
        show(base, base + "#" + fragment);
        return true;
    }

    private static boolean valid(Uri uri) {
        if (uri == null || !"http".equals(uri.getScheme()) || uri.getHost() == null || !isLocal(uri.getHost())) return false;
        String fragment = uri.getFragment();
        return fragment != null && fragment.matches("pair=[A-Za-z0-9]{4,16}");
    }

    /**
     * A pairing link opened from outside the app: the QR code read by the
     * phone's own camera ({@code http://…/#pair=CODE}), or the "open in the
     * app" button of the dashboard in a browser
     * ({@code streamphone://pair?h=192.168.0.10:47800&c=CODE}). Anything can
     * send such a link, so the app asks before it connects anywhere.
     */
    private void open(Intent intent) {
        Uri uri = intent == null || !Intent.ACTION_VIEW.equals(intent.getAction()) ? null : intent.getData();
        if (uri == null) return;
        // Once only: not again when the screen is rebuilt.
        intent.setData(null);

        String built;
        try {
            if ("streamphone".equals(uri.getScheme())) {
                built = "http://" + uri.getQueryParameter("h") + "/#pair=" + uri.getQueryParameter("c");
            } else {
                built = uri.toString();
            }
        } catch (RuntimeException malformed) {
            built = null;
        }
        final String link = built;
        final Uri target = link == null ? null : Uri.parse(link);
        if (!valid(target)) {
            Toast.makeText(this, R.string.bad_link, Toast.LENGTH_LONG).show();
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle(R.string.link_title)
                .setMessage(target.getHost())
                .setNegativeButton(R.string.cancel, null)
                .setPositiveButton(R.string.connect, (dialog, which) -> pair(link))
                .show();
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
        // The manifest already keeps the scanner in landscape, like the app.
        scanner.setOrientationLocked(false);
        try {
            scanner.initiateScan();
        } catch (RuntimeException failed) {
            // No scanner on this phone: the address can still be typed in.
            Toast.makeText(this, R.string.scan_failed, Toast.LENGTH_LONG).show();
            typeAddress();
        }
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        IntentResult scanned = IntentIntegrator.parseActivityResult(request, result, data);
        if (scanned == null) {
            super.onActivityResult(request, result, data);
            return;
        }
        if (scanned.getContents() == null) return;
        boolean paired;
        try {
            paired = pair(scanned.getContents());
        } catch (RuntimeException unreadable) {
            paired = false;
        }
        if (!paired) Toast.makeText(this, R.string.bad_link, Toast.LENGTH_LONG).show();
    }

    // ---- when the app closes by itself ---------------------------------------

    /**
     * Keeps the reason of a crash, from any thread, so the next start can show
     * it: on a phone there is nowhere else to read it.
     */
    private void rememberCrashes() {
        final Thread.UncaughtExceptionHandler system = Thread.getDefaultUncaughtExceptionHandler();
        if (system instanceof CrashKeeper) return;
        Thread.setDefaultUncaughtExceptionHandler(new CrashKeeper(prefs, system));
    }

    private static final class CrashKeeper implements Thread.UncaughtExceptionHandler {
        private final SharedPreferences prefs;
        private final Thread.UncaughtExceptionHandler next;

        CrashKeeper(SharedPreferences prefs, Thread.UncaughtExceptionHandler next) {
            this.prefs = prefs;
            this.next = next;
        }

        @Override
        @SuppressLint("ApplySharedPref")
        public void uncaughtException(Thread thread, Throwable error) {
            try {
                StringWriter text = new StringWriter();
                error.printStackTrace(new PrintWriter(text));
                String report = "StreamPhone " + BuildConfig.VERSION_NAME + ", Android " + android.os.Build.VERSION.RELEASE
                        + ", " + android.os.Build.MANUFACTURER + " " + android.os.Build.MODEL + "\n" + text;
                // commit, not apply: the process is about to end.
                prefs.edit().putString(KEY_CRASH, report.length() > 6000 ? report.substring(0, 6000) : report).commit();
            } catch (Throwable ignored) {
                // Nothing more can be done here.
            }
            if (next != null) next.uncaughtException(thread, error);
        }
    }

    private void copyCrash() {
        String report = prefs.getString(KEY_CRASH, null);
        if (report == null) return;
        ClipboardManager clipboard = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        if (clipboard != null) clipboard.setPrimaryClip(ClipData.newPlainText("StreamPhone", report));
        Toast.makeText(this, R.string.crash_copied, Toast.LENGTH_SHORT).show();
        prefs.edit().remove(KEY_CRASH).apply();
        crashed.setVisibility(View.GONE);
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

        // Shown after the app closed by itself; a tap copies the details.
        String report = prefs.getString(KEY_CRASH, null);
        crashed = text(R.string.crashed, 13, 0xFFFF9F0A);
        crashed.setGravity(Gravity.CENTER);
        crashed.setPadding(0, dp(18), 0, 0);
        crashed.setMaxWidth(dp(560));
        crashed.setMaxLines(3);
        if (report != null) {
            String[] lines = report.split("\n");
            crashed.setText(getString(R.string.crashed) + "\n" + (lines.length > 1 ? lines[1] : ""));
        }
        crashed.setVisibility(report != null ? View.VISIBLE : View.GONE);
        crashed.setOnClickListener(v -> copyCrash());
        box.addView(crashed);
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
