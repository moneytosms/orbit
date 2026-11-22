// Posts reminder notifications with Complete, Snooze and Open actions.
package dev.orbit.app

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import org.json.JSONObject
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat

object Notifier {
    private const val CHANNEL = "reminders"
    private const val UNDO_NOTIFICATION = 0x0b17
    private val FLAGS = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

    /** Idempotent: Android keeps the channel once created. */
    fun createChannel(ctx: Context) {
        val channel = NotificationChannel(
            CHANNEL,
            ctx.getString(R.string.reminders_channel),
            NotificationManager.IMPORTANCE_HIGH,
        )
        channel.enableVibration(true)
        channel.vibrationPattern = longArrayOf(0, 8)
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

    /** One replaceable completion receipt with an Undo action for notification-only use. */
    fun showUndo(ctx: Context, taskId: String, receipt: JSONObject) {
        createChannel(ctx)
        val undo = Intent(ctx, ReminderActionReceiver::class.java)
            .setAction(ReminderActionReceiver.ACTION_UNDO)
            .putExtra(ReminderActionReceiver.EXTRA_TASK_ID, taskId)
            .putExtra(ReminderActionReceiver.EXTRA_UNDO, receipt.toString())
            .putExtra(ReminderActionReceiver.EXTRA_NOTIFICATION_ID, UNDO_NOTIFICATION)
        val action = PendingIntent.getBroadcast(ctx, UNDO_NOTIFICATION, undo, FLAGS)
        val notification = NotificationCompat.Builder(ctx, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(ctx.getString(R.string.task_completed))
            .setContentText(ctx.getString(R.string.undo_available))
            .setOnlyAlertOnce(true)
            .setAutoCancel(true)
            .setTimeoutAfter(5_000)
            .addAction(0, ctx.getString(R.string.undo_action), action)
            .build()
        NotificationManagerCompat.from(ctx).notify(UNDO_NOTIFICATION, notification)
    }

    private fun action(ctx: Context, taskId: String, key: String, id: Int, act: String): PendingIntent {
        val i = Intent(ctx, ReminderActionReceiver::class.java)
            .setAction(act)
            .putExtra(ReminderActionReceiver.EXTRA_TASK_ID, taskId)
            .putExtra(ReminderActionReceiver.EXTRA_NOTIFICATION_ID, id)
        return PendingIntent.getBroadcast(ctx, (key + act).hashCode(), i, FLAGS)
    }
}
