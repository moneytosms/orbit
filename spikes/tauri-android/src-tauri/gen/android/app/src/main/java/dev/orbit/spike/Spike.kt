// PROTOTYPE: the whole native Kotlin module of the spike in one file.
package dev.orbit.spike

import android.app.*
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.*
import android.os.Bundle
import android.util.Log
import android.widget.*
import androidx.work.*
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

const val TAG = "OrbitSpike"
const val SERVER = "http://10.0.2.2:8787/sync" // host machine from the emulator

// ---- Outbox: one JSON file per command (temp + fsync + atomic rename), per issue #18 ----
object Outbox {
    fun write(ctx: Context, type: String, args: JSONObject, source: String) {
        val dir = File(ctx.filesDir, "outbox").apply { mkdirs() }
        val uuid = UUID.randomUUID().toString()
        val cmd = JSONObject().put("uuid", uuid).put("type", type).put("args", args)
            .put("source", source).put("created", System.currentTimeMillis())
        val tmp = File(dir, "$uuid.tmp")
        FileOutputStream(tmp).use { it.write(cmd.toString().toByteArray()); it.fd.sync() }
        tmp.renameTo(File(dir, "$uuid.json"))
        Log.i(TAG, "outbox write $uuid $source")
        flushLater(ctx)
    }

    fun flushLater(ctx: Context) {
        val req = OneTimeWorkRequestBuilder<FlushWorker>()
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .build()
        WorkManager.getInstance(ctx).enqueueUniqueWork("flush", ExistingWorkPolicy.APPEND_OR_REPLACE, req)
    }
}

class FlushWorker(ctx: Context, p: WorkerParameters) : Worker(ctx, p) {
    override fun doWork(): Result {
        val dir = File(applicationContext.filesDir, "outbox")
        val sent = File(applicationContext.filesDir, "sent").apply { mkdirs() }
        for (f in dir.listFiles { x -> x.name.endsWith(".json") }.orEmpty().sortedBy { it.name }) {
            try {
                val c = URL(SERVER).openConnection() as HttpURLConnection
                c.requestMethod = "POST"; c.doOutput = true; c.connectTimeout = 10_000
                c.setRequestProperty("Content-Type", "application/json")
                c.outputStream.use { it.write(f.readBytes()) }
                if (c.responseCode != 200) return Result.retry()
                f.renameTo(File(sent, f.name))
                Log.i(TAG, "flushed ${f.name}")
            } catch (e: Exception) {
                Log.w(TAG, "flush failed: $e"); return Result.retry()
            }
        }
        return Result.success()
    }
}

// ---- Capture: widget tap and share target both land here ----
class CaptureActivity : Activity() {
    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        val shared = intent?.takeIf { it.action == Intent.ACTION_SEND }?.getStringExtra(Intent.EXTRA_TEXT)
        if (shared != null) { // share target: capture without UI
            Outbox.write(this, "task_add", JSONObject().put("content", shared), "share")
            Toast.makeText(this, "Added to Orbit", Toast.LENGTH_SHORT).show()
            finish(); return
        }
        val input = EditText(this).apply { hint = "Add task"; isSingleLine = true }
        val add = Button(this).apply { text = "Add" }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; setPadding(32, 32, 32, 32); addView(input); addView(add)
        })
        add.setOnClickListener {
            Outbox.write(this, "task_add", JSONObject().put("content", input.text.toString()), "widget")
            finish()
        }
    }
}

class CaptureWidget : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        val pi = PendingIntent.getActivity(ctx, 0, Intent(ctx, CaptureActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE)
        val views = android.widget.RemoteViews(ctx.packageName, R.layout.widget).apply {
            setOnClickPendingIntent(R.id.widget_add, pi)
        }
        ids.forEach { mgr.updateAppWidget(it, views) }
    }
}

// ---- Reminders: setAlarmClock, persisted so BootReceiver can re-register (issue #8) ----
object Alarms {
    private fun prefs(ctx: Context) = ctx.getSharedPreferences("alarms", Context.MODE_PRIVATE)
    fun all(ctx: Context): Map<String, JSONObject> =
        prefs(ctx).all.mapValues { JSONObject(it.value as String) }

    fun schedule(ctx: Context, id: String, at: Long, title: String) {
        prefs(ctx).edit().putString(id, JSONObject().put("at", at).put("title", title).toString()).commit()
        arm(ctx, id, at)
    }

    fun arm(ctx: Context, id: String, at: Long) {
        val fire = PendingIntent.getBroadcast(ctx, id.hashCode(),
            Intent(ctx, AlarmReceiver::class.java).putExtra("id", id), PendingIntent.FLAG_IMMUTABLE)
        val show = PendingIntent.getActivity(ctx, 0, Intent(ctx, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        ctx.getSystemService(AlarmManager::class.java).setAlarmClock(AlarmManager.AlarmClockInfo(at, show), fire)
        Log.i(TAG, "armed $id at $at")
    }

    fun fired(ctx: Context, id: String) = prefs(ctx).edit().remove(id).commit()
}

class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, i: Intent) {
        val id = i.getStringExtra("id") ?: return
        val title = Alarms.all(ctx)[id]?.optString("title") ?: "Reminder"
        val nm = ctx.getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel("reminders", "Reminders", NotificationManager.IMPORTANCE_HIGH))
        nm.notify(id.hashCode(), Notification.Builder(ctx, "reminders")
            .setSmallIcon(android.R.drawable.ic_lock_idle_alarm).setContentTitle(title)
            .setContentText("fired ${java.util.Date()}").build())
        Alarms.fired(ctx, id)
        Log.i(TAG, "fired $id")
    }
}

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, i: Intent) {
        if (i.action != Intent.ACTION_BOOT_COMPLETED) return
        Alarms.all(ctx).forEach { (id, a) -> Alarms.arm(ctx, id, maxOf(a.getLong("at"), System.currentTimeMillis() + 1000)) }
        Outbox.flushLater(ctx)
        Log.i(TAG, "boot: re-armed ${Alarms.all(ctx).size}")
    }
}

// ---- Tauri bridge: Rust calls these via run_mobile_plugin ----
@InvokeArg
class ScheduleArgs { var secs: Long = 60; var title: String = "Reminder" }

@TauriPlugin
class OrbitPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun schedule(invoke: Invoke) {
        val a = invoke.parseArgs(ScheduleArgs::class.java)
        val id = UUID.randomUUID().toString()
        val at = System.currentTimeMillis() + a.secs * 1000
        Alarms.schedule(activity, id, at, a.title)
        invoke.resolve(JSObject().put("id", id).put("at", at))
    }

    @Command
    fun alarms(invoke: Invoke) {
        val arr = JSArray()
        Alarms.all(activity).forEach { (id, a) -> arr.put(JSObject().put("id", id).put("at", a.getLong("at")).put("title", a.getString("title"))) }
        invoke.resolve(JSObject().put("alarms", arr))
    }
}
