package org.ingot.server

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import java.io.File

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
    // Keep the renderer alive while the app is in the background. Otherwise Android
    // may kill it to reclaim memory and the whole page reloads when the user returns.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      webView.setRendererPriorityPolicy(WebView.RENDERER_PRIORITY_IMPORTANT, false)
    }
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

  /** Opens the system share sheet with plain text (e.g. a server invite) */
  @JavascriptInterface
  fun shareText(title: String, text: String) {
    val send = Intent(Intent.ACTION_SEND).apply {
      type = "text/plain"
      putExtra(Intent.EXTRA_SUBJECT, title)
      putExtra(Intent.EXTRA_TEXT, text)
    }
    context.startActivity(
      Intent.createChooser(send, title).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    )
  }

  /** Shares a file the app exported into its cache folder (e.g. a server backup) */
  @JavascriptInterface
  fun shareFile(path: String, mimeType: String, title: String) {
    val file = File(path).canonicalFile
    // Only files the app itself wrote to its cache may be handed to other apps
    if (!file.path.startsWith(context.cacheDir.canonicalPath + File.separator) || !file.isFile) return
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
    val send = Intent(Intent.ACTION_SEND).apply {
      type = mimeType
      putExtra(Intent.EXTRA_STREAM, uri)
      putExtra(Intent.EXTRA_SUBJECT, title)
      addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }
    context.startActivity(
      Intent.createChooser(send, title)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
    )
  }
}
