package dev.orbit.app

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import android.webkit.WebView
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private var web: WebView? = null

  override fun onWebViewCreate(webView: WebView) {
    web = webView
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // The content root, not the WebView: a WebView ignores its own padding, so pages drew under the bars.
    findViewById<View>(android.R.id.content).padForSystemBars(behindNavBar = true)
    Link.set(intent)
    Sync.schedule(this)
    WidgetRefresh.schedule(this)
    askNotificationPermission()
    BatteryGuidance.maybeShow(this)
    ShellUpdate.check(this)
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    // A running page gets the link at once; a page still loading finds it with `take_link`.
    val link = Link.set(intent) ?: return
    val page = web ?: return
    Link.take()
    val js = if (link.startsWith("#/new")) "window.dispatchEvent(new CustomEvent('orbit-link', { detail: '$link' }))"
    else "location.hash = '$link'"
    page.post { page.evaluateJavascript(js, null) }
  }

  override fun onStart() {
    super.onStart()
    ReminderScheduler.rearmAsync(this)
    // Opening the app repaints widgets that Android blanked (force-stop, update).
    Thread { WidgetRefresh.all(applicationContext) }.start()
  }

  /** Android 13+ asks before any notification is shown; reminders need it. */
  private fun askNotificationPermission() {
    if (Build.VERSION.SDK_INT < 33) return
    val perm = "android.permission.POST_NOTIFICATIONS"
    if (checkSelfPermission(perm) != PackageManager.PERMISSION_GRANTED) requestPermissions(arrayOf(perm), 69)
  }
}

/** What a widget tap asked the app to open, as a web hash route, until the page takes it. */
object Link {
  @Volatile private var pending: String? = null
  private val id = Regex("[0-9a-f-]{36}")
  const val EXTRA_CAPTURE = "orbit_capture"

  /** Opens the app's own capture window (the one the + button uses); `prefill` is a source's text such as `@Home`. */
  fun capture(ctx: Context, prefill: String): Intent =
    Intent(ctx, MainActivity::class.java).putExtra(EXTRA_CAPTURE, prefill).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

  /** The hash route a widget tap asked for, or null. Ids are checked: the extra is not ours to trust. */
  fun set(intent: Intent?): String? {
    val link = intent?.getStringExtra(ListWidget.EXTRA_LINK)?.removePrefix("orbit://task/")
    val goal = intent?.getStringExtra(GoalWidgetProvider.EXTRA_OPEN)
    val capture = intent?.getStringExtra(EXTRA_CAPTURE)
    val route = when {
      // The text rides in the route percent-encoded, so it cannot break out of the JS string it is handed to.
      capture != null -> "#/new?q=" + Uri.encode(capture.take(200)).replace("'", "%27")
      link != null && id.matches(link) -> "#/task/$link"
      goal != null && goal.startsWith("goal:") && id.matches(goal.removePrefix("goal:")) -> "#/goals"
      else -> return null
    }
    pending = route
    return route
  }

  fun take(): String = pending.also { pending = null } ?: ""
}
