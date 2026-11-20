// Arms one exact alarm for the soonest reminder and shows any reminder already due.
package dev.orbit.app

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.util.Log
import org.json.JSONArray
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter

object ReminderScheduler {
    /** The local wall-clock format the Rust core uses: `YYYY-MM-DDTHH:MM`. */
    val LOCAL: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm")
    private const val PREFS = "orbit-reminders"
    private const val SHOWN = "shown"

    /** One reminder from `Core.reminders`; `key` identifies it across re-arms. */
    class Fire(val key: String, val at: String, val title: String, val taskId: String?)

    /** Shows what is due, then arms the soonest future reminder (or cancels the alarm if none). */
    @Synchronized
    fun rearm(ctx: Context) {
        val now = LocalDateTime.now().format(LOCAL)
        val fires = fires(ctx, now) ?: return
        val prefs = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val shown = prefs.getStringSet(SHOWN, null)?.toMutableSet() ?: mutableSetOf()
        shown.retainAll(fires.map { it.key }.toSet())
        var next: Fire? = null
        for (f in fires) {
            if (f.key in shown) continue
            if (f.at <= now) {
                Notifier.show(ctx, f)
                shown.add(f.key)
            } else if (next == null || f.at < next.at) {
                next = f
            }
        }
        prefs.edit().putStringSet(SHOWN, shown).apply()
        arm(ctx, next)
    }

    /** Runs [rearm] off the caller's thread, so JNI into Rust never blocks the UI. */
    fun rearmAsync(ctx: Context) {
        Thread {
            try {
                rearm(ctx.applicationContext)
            } catch (e: Exception) {
                Log.w(TAG, "rearm: $e")
            }
        }.start()
    }

    private fun fires(ctx: Context, now: String): List<Fire>? =
        try {
            val arr = JSONArray(Core.reminders(Core.dir(ctx), now))
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                // `late` is left out of the key: it flips as time passes, the reminder does not.
                val key = listOf("task_id", "reminder_id", "routine_id", "at")
                    .joinToString("|") { if (o.isNull(it)) "" else o.getString(it) }
                Fire(
                    key,
                    o.getString("at"),
                    o.getString("title"),
                    if (o.isNull("task_id")) null else o.getString("task_id"),
                )
            }
        } catch (e: Exception) {
            Log.w(TAG, "reminders: $e")
            null
        }

    private fun arm(ctx: Context, next: Fire?) {
        val am = ctx.getSystemService(AlarmManager::class.java)
        val flags = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        val fire = PendingIntent.getBroadcast(ctx, 0, Intent(ctx, AlarmReceiver::class.java), flags)
        if (next == null) {
            am.cancel(fire)
            return
        }
        val millis = LocalDateTime.parse(next.at).atZone(ZoneId.systemDefault()).toInstant().toEpochMilli()
        val show = PendingIntent.getActivity(ctx, 0, Intent(ctx, MainActivity::class.java), flags)
        am.setAlarmClock(AlarmManager.AlarmClockInfo(millis, show), fire)
    }
}
