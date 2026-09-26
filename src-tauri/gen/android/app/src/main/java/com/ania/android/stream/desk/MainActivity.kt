package com.ania.android.stream.desk

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import android.webkit.JavascriptInterface
import android.view.WindowManager
import android.webkit.WebView
import android.widget.Toast
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private var wifiLock: WifiManager.WifiLock? = null
  // The frontend flips this off when it connects to 127.0.0.1 (USB mode via
  // `adb reverse`), where a Wi-Fi performance lock is pure battery drain.
  @Volatile private var wifiLockWanted = true

  // Screen behaviour mirrors the client's settings (Keep Screen On, Show Over
  // Lock Screen). The WebView pushes them through AndroidScreen; they are
  // persisted here too so a cold start (e.g. the Companion launching us over
  // USB) applies them before the page has loaded.
  private val screenPrefs by lazy { getSharedPreferences("screen", Context.MODE_PRIVATE) }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // When the Companion launches us over USB the phone may be dark on the
    // desk: turn the screen on. Without Show Over Lock Screen the user still
    // unlocks as usual.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
      setTurnScreenOn(true)
    } else {
      @Suppress("DEPRECATION")
      window.addFlags(WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON)
    }
    applyKeepScreenOn(screenPrefs.getBoolean(PREF_KEEP_SCREEN_ON, false))
    applyShowWhenLocked(screenPrefs.getBoolean(PREF_SHOW_WHEN_LOCKED, false))
  }

  // Window-scoped: needs no WAKE_LOCK permission and releases itself the
  // moment the user switches away.
  private fun applyKeepScreenOn(enabled: Boolean) {
    if (enabled) {
      window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    } else {
      window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
  }

  // Opt-in (default off): makes the grid usable without unlocking the phone,
  // which suits a dedicated desk deck but not a personal phone.
  private fun applyShowWhenLocked(enabled: Boolean) {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
      setShowWhenLocked(enabled)
    } else if (enabled) {
      @Suppress("DEPRECATION")
      window.addFlags(WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED)
    } else {
      @Suppress("DEPRECATION")
      window.clearFlags(WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED)
    }
  }

  // Expose a native orientation toggle to the WebView. Runtime orientation can't
  // go through Rust JNI (it panicked on the tao event-loop thread and, with
  // panic=abort, crashed the process), so the frontend calls
  // window.AndroidOrientation.setOrientation(mode). `mode` is an
  // ActivityInfo.SCREEN_ORIENTATION_* constant matching ANDROID_ORIENTATION in
  // ClientView.vue (-1 unspecified, 0 landscape, 1 portrait, 8 reverse-landscape).
  override fun onWebViewCreate(webView: WebView) {
    // Debug builds only: exposes the WebView to `chrome://inspect` /
    // `adb forward ... webview_devtools_remote_<pid>` so the client's JS can
    // be inspected on a real device. Never enabled for a release build.
    if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
      WebView.setWebContentsDebuggingEnabled(true)
    }
    webView.addJavascriptInterface(OrientationBridge(), "AndroidOrientation")
    webView.addJavascriptInterface(WifiLockBridge(), "AndroidWifiLock")
    webView.addJavascriptInterface(ScreenBridge(), "AndroidScreen")
  }

  inner class ScreenBridge {
    @JavascriptInterface
    fun setKeepScreenOn(enabled: Boolean) {
      screenPrefs.edit().putBoolean(PREF_KEEP_SCREEN_ON, enabled).apply()
      runOnUiThread { applyKeepScreenOn(enabled) }
    }

    @JavascriptInterface
    fun setShowWhenLocked(enabled: Boolean) {
      screenPrefs.edit().putBoolean(PREF_SHOW_WHEN_LOCKED, enabled).apply()
      runOnUiThread { applyShowWhenLocked(enabled) }
    }
  }

  inner class WifiLockBridge {
    @JavascriptInterface
    fun setEnabled(enabled: Boolean) {
      wifiLockWanted = enabled
      runOnUiThread { if (enabled) acquireWifiLock() else releaseWifiLock() }
    }
  }

  inner class OrientationBridge {
    @JavascriptInterface
    fun setOrientation(mode: Int) {
      // @JavascriptInterface runs on a binder thread; setRequestedOrientation
      // must touch the Activity on the UI thread.
      runOnUiThread { requestedOrientation = mode }
    }
  }

  override fun onResume() {
    super.onResume()
    acquireWifiLock()
    requestBatteryOptimizationExemptionOnce()
  }

  override fun onStop() {
    releaseWifiLock()
    super.onStop()
  }

  override fun onDestroy() {
    releaseWifiLock()
    super.onDestroy()
  }

  private fun acquireWifiLock() {
    if (!wifiLockWanted) return
    if (wifiLock?.isHeld == true) return
    try {
      val wm = applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return
      val mode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        WifiManager.WIFI_MODE_FULL_LOW_LATENCY
      } else {
        @Suppress("DEPRECATION")
        WifiManager.WIFI_MODE_FULL_HIGH_PERF
      }
      wifiLock = wm.createWifiLock(mode, "android_stream_desk:wifi").also { it.acquire() }
    } catch (_: Exception) {
      wifiLock = null
    }
  }

  private fun releaseWifiLock() {
    try { wifiLock?.let { if (it.isHeld) it.release() } } catch (_: Exception) {}
    wifiLock = null
  }

  // Android/MIUI battery optimization kills WiFi on battery when screen wake lock is held.
  // Prompt once to exclude this app from battery optimization.
  private fun requestBatteryOptimizationExemptionOnce() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) return
    val pm = getSystemService(Context.POWER_SERVICE) as? PowerManager ?: return
    if (pm.isIgnoringBatteryOptimizations(packageName)) return
    val prefs = getPreferences(Context.MODE_PRIVATE)
    if (prefs.getBoolean("battery_opt_asked", false)) return
    prefs.edit().putBoolean("battery_opt_asked", true).apply()
    Toast.makeText(
      this,
      "Allow \"Unrestricted battery\" to keep the Wi-Fi connection alive on battery",
      Toast.LENGTH_LONG
    ).show()
    try {
      startActivity(
        Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
          data = Uri.parse("package:$packageName")
        }
      )
    } catch (_: Exception) {}
  }

  companion object {
    private const val PREF_KEEP_SCREEN_ON = "keep_screen_on"
    private const val PREF_SHOW_WHEN_LOCKED = "show_when_locked"
  }
}
