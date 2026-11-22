// Handles the Complete and Snooze buttons on a reminder notification.
package dev.orbit.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.core.app.NotificationManagerCompat
import java.time.LocalDate
import java.time.LocalDateTime

class ReminderActionReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val action = intent.action ?: return
        val taskId = intent.getStringExtra(EXTRA_TASK_ID)
        val receipt = intent.getStringExtra(EXTRA_UNDO)
        if (action == ACTION_UNDO && receipt == null) return
        if (action != ACTION_UNDO && taskId == null) return
        val notificationId = intent.getIntExtra(EXTRA_NOTIFICATION_ID, 0)
        val result = goAsync()
        Thread {
            try {
                NotificationManagerCompat.from(ctx).cancel(notificationId)
                val dir = Core.dir(ctx)
                val r = when (action) {
                    ACTION_COMPLETE -> {
                        val response = org.json.JSONObject(Core.complete(dir, taskId!!, LocalDate.now().toString()))
                        if (response.optBoolean("completed")) {
                            response.optJSONObject("undo")?.let { Notifier.showUndo(ctx, taskId, it) }
                        }
                        response.toString()
                    }
                    ACTION_UNDO -> Core.uncomplete(dir, receipt!!)
                    ACTION_SNOOZE -> Core.reminder_snooze(
                        dir,
                        taskId!!,
                        LocalDateTime.now().plusMinutes(SNOOZE_MINUTES).format(ReminderScheduler.LOCAL),
                    )
                    else -> return@Thread
                }
                Log.i(TAG, "reminder $action: $r")
                ReminderScheduler.rearm(ctx.applicationContext)
                WidgetRefresh.all(ctx.applicationContext)
                Sync.now(ctx.applicationContext)
            } catch (e: Exception) {
                Log.w(TAG, "reminder action: $e")
            } finally {
                result.finish()
            }
        }.start()
    }

    companion object {
        const val ACTION_COMPLETE = "dev.orbit.app.REMINDER_COMPLETE"
        const val ACTION_UNDO = "dev.orbit.app.TASK_UNDO"
        const val ACTION_SNOOZE = "dev.orbit.app.REMINDER_SNOOZE"
        const val EXTRA_TASK_ID = "task_id"
        const val EXTRA_UNDO = "task_undo"
        const val EXTRA_NOTIFICATION_ID = "notification_id"
        private const val SNOOZE_MINUTES = 10L
    }
}
