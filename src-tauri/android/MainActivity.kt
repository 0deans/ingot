package org.ingot.server

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT)
    )
    super.onCreate(savedInstanceState)

    try {
      val libDir = applicationInfo.nativeLibraryDir
      android.system.Os.setenv("ANDROID_APP_LIB_DIR", libDir, true)
    } catch (_: Exception) {}

    // Needed to show the foreground-service notification on Android 13+
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
      checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
    }

    val contentView = findViewById<android.view.View>(android.R.id.content)
    if (contentView != null) {
      ViewCompat.setOnApplyWindowInsetsListener(contentView) { view, windowInsets ->
        val insets = windowInsets.getInsets(
          WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
        )
        view.setPadding(insets.left, insets.top, insets.right, insets.bottom)
        windowInsets
      }
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.setBackgroundColor(Color.parseColor("#09090b"))
    webView.addJavascriptInterface(HostBridge(applicationContext), "IngotHost")
  }
}

/**
 * Exposed to the frontend as `window.IngotHost`. While a server or tunnel is active the
 * UI keeps [ServerHostService] running; without a foreground service Android treats the
 * app as cached once it leaves the screen and cuts off its network access.
 */
class HostBridge(private val context: Context) {
  @JavascriptInterface
  fun start(title: String, detail: String) {
    ServerHostService.start(context, title, detail)
  }

  @JavascriptInterface
  fun stop() {
    ServerHostService.stop(context)
  }
}
