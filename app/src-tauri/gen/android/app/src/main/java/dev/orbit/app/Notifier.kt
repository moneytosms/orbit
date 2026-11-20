// Posts reminder notifications with Complete, Snooze and Open actions.
package dev.orbit.app

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat

object Notifier {
    private const val CHANNEL = "reminders"
    private val FLAGS = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

    /** Idempotent: Android keeps the channel once created. */
    fun createChannel(ctx: Context) {
        val channel = NotificationChannel(
            CHANNEL,
            ctx.getString(R.string.reminders_channel),
            NotificationManager.IMPORTANCE_HIGH,
        )
        ctx.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    fun show(ctx: Context, f: ReminderScheduler.Fire) {
        createChannel(ctx)
        val id = f.key.hashCode()
        val open = PendingIntent.getActivity(ctx, id, Intent(ctx, MainActivity::class.java), FLAGS)
        val b = NotificationCompat.Builder(ctx, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(f.title)
            .setContentText(f.at.replace('T', ' '))
            .setContentIntent(open)
            .setAutoCancel(true)
        f.taskId?.let { taskId ->
            b.addAction(0, ctx.getString(R.string.reminder_complete), action(ctx, taskId, f.key, id, ReminderActionReceiver.ACTION_COMPLETE))
            b.addAction(0, ctx.getString(R.string.reminder_snooze), action(ctx, taskId, f.key, id, ReminderActionReceiver.ACTION_SNOOZE))
        }
        b.addAction(0, ctx.getString(R.string.reminder_open), open)
        NotificationManagerCompat.from(ctx).notify(id, b.build())
    }

    private fun action(ctx: Context, taskId: String, key: String, id: Int, act: String): PendingIntent {
        val i = Intent(ctx, ReminderActionReceiver::class.java)
            .setAction(act)
            .putExtra(ReminderActionReceiver.EXTRA_TASK_ID, taskId)
            .putExtra(ReminderActionReceiver.EXTRA_NOTIFICATION_ID, id)
        return PendingIntent.getBroadcast(ctx, (key + act).hashCode(), i, FLAGS)
    }
}
