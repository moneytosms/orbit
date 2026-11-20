// Fires when the armed reminder is due: shows it and arms the next one.
package dev.orbit.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val result = goAsync()
        Thread {
            try {
                ReminderScheduler.rearm(ctx.applicationContext)
            } catch (e: Exception) {
                Log.w(TAG, "alarm: $e")
            } finally {
                result.finish()
            }
        }.start()
    }
}
