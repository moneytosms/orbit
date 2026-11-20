package dev.orbit.app

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    Sync.schedule(this)
    WidgetRefresh.schedule(this)
    askNotificationPermission()
    BatteryGuidance.maybeShow(this)
  }

  override fun onStart() {
    super.onStart()
    ReminderScheduler.rearmAsync(this)
  }

  /** Android 13+ asks before any notification is shown; reminders need it. */
  private fun askNotificationPermission() {
    if (Build.VERSION.SDK_INT < 33) return
    val perm = "android.permission.POST_NOTIFICATIONS"
    if (checkSelfPermission(perm) != PackageManager.PERMISSION_GRANTED) requestPermissions(arrayOf(perm), 69)
  }
}
