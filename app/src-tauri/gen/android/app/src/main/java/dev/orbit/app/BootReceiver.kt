// Alarms do not survive a reboot, so the next reminder is armed again on boot. After an app update the
// widgets are redrawn too, since they show a blank placeholder until something repaints them.
package dev.orbit.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        when (intent.action) {
            Intent.ACTION_BOOT_COMPLETED -> {}
            Intent.ACTION_MY_PACKAGE_REPLACED -> WidgetRefresh.all(ctx.applicationContext)
            else -> return
        }
        val result = goAsync()
        Thread {
            try {
                ReminderScheduler.rearm(ctx.applicationContext)
            } catch (e: Exception) {
                Log.w(TAG, "boot: $e")
            } finally {
                result.finish()
            }
        }.start()
    }
}
