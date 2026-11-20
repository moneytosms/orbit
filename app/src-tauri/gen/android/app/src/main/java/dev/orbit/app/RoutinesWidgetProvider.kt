// #71 Routines widget: today's unchecked Routines as a RemoteCollectionItems list. A tap checks in after a 2 s undo window.
package dev.orbit.app

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.graphics.Paint
import android.util.Log
import android.view.View
import android.widget.RemoteViews
import androidx.work.*
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import java.time.LocalDate
import java.util.concurrent.TimeUnit

class RoutinesWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) = render(ctx, mgr, ids)

    override fun onReceive(ctx: Context, intent: Intent) {
        if (intent.action != ACTION_TAP) {
            super.onReceive(ctx, intent)
            return
        }
        val id = intent.getStringExtra(EXTRA_ROUTINE_ID) ?: return
        RoutinesPending.toggle(ctx, id)
        WidgetRefresh.all(ctx)
    }

    override fun onDeleted(ctx: Context, ids: IntArray) {
        val edit = prefs(ctx).edit()
        for (id in ids) edit.remove("area_$id")
        edit.apply()
    }

    companion object {
        const val ACTION_TAP = "dev.orbit.app.ROUTINE_TAP"
        const val EXTRA_ROUTINE_ID = "routine_id"
        private const val PREFS = "routines_widgets"

        fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

        /** Redraws the given widgets from the replica. The config activity calls this after a choice. */
        fun render(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
            val dir = Core.dir(ctx)
            val today = WidgetData.today()
            val pending = RoutinesPending.ids(ctx)
            // Click on a row: the template is combined with the row's fill-in intent (which carries the routine id).
            val template = PendingIntent.getBroadcast(
                ctx, 0, Intent(ctx, RoutinesWidgetProvider::class.java).setAction(ACTION_TAP),
                PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            for (widget in ids) {
                val area = prefs(ctx).getString("area_$widget", "") ?: ""
                val items = RemoteViews.RemoteCollectionItems.Builder().setHasStableIds(true).setViewTypeCount(1)
                var count = 0
                for (r in WidgetData.objects(Core.routines(dir, today, area))) {
                    if (!WidgetData.unchecked(r, today)) continue
                    val rid = r.getString("id")
                    val isPending = rid in pending
                    val row = RemoteViews(ctx.packageName, R.layout.widget_routines_row)
                    row.setTextViewText(R.id.name, r.getString("name"))
                    row.setTextViewText(
                        R.id.streak,
                        ctx.getString(R.string.routines_streak, r.getJSONObject("stats").optInt("current_streak"))
                    )
                    row.setInt(
                        R.id.name, "setPaintFlags",
                        Paint.ANTI_ALIAS_FLAG or (if (isPending) Paint.STRIKE_THRU_TEXT_FLAG else 0)
                    )
                    row.setImageViewResource(R.id.check, if (isPending) R.drawable.filled_circle else R.drawable.ring)
                    row.setViewVisibility(R.id.undo, if (isPending) View.VISIBLE else View.GONE)
                    row.setOnClickFillInIntent(R.id.row, Intent().putExtra(EXTRA_ROUTINE_ID, rid))
                    items.addItem(rid.hashCode().toLong(), row)
                    count++
                }
                val views = RemoteViews(ctx.packageName, R.layout.widget_routines)
                views.setRemoteAdapter(R.id.list, items.build())
                views.setPendingIntentTemplate(R.id.list, template)
                views.setViewVisibility(R.id.list, if (count == 0) View.GONE else View.VISIBLE)
                views.setViewVisibility(R.id.empty, if (count == 0) View.VISIBLE else View.GONE)
                views.setTextViewText(R.id.left, ctx.getString(R.string.routines_left, count))
                views.setViewVisibility(R.id.left, if (count == 0) View.GONE else View.VISIBLE)
                mgr.updateAppWidget(widget, views)
            }
        }
    }
}

/** The ids of Routines tapped but not yet checked in. Kept in the widget prefs so a process death keeps the undo state. */
internal object RoutinesPending {
    private const val KEY = "pending"

    fun ids(ctx: Context): Set<String> =
        RoutinesWidgetProvider.prefs(ctx).getStringSet(KEY, emptySet()) ?: emptySet()

    /** The first tap marks the Routine pending and schedules the check-in 2 s out; a tap while pending undoes it. */
    fun toggle(ctx: Context, id: String) {
        val wm = WorkManager.getInstance(ctx)
        if (id in ids(ctx)) {
            wm.cancelUniqueWork(workName(id))
            set(ctx, id, false)
        } else {
            set(ctx, id, true)
            val req = OneTimeWorkRequestBuilder<RoutinesCommitWorker>()
                .setInitialDelay(ListWidget.UNDO_MS, TimeUnit.MILLISECONDS)
                .setInputData(Data.Builder().putString(RoutinesCommitWorker.KEY_ID, id).build())
                .build()
            wm.enqueueUniqueWork(workName(id), ExistingWorkPolicy.REPLACE, req)
        }
    }

    fun set(ctx: Context, id: String, on: Boolean) {
        val s = HashSet(ids(ctx))
        if (on) s.add(id) else s.remove(id)
        RoutinesWidgetProvider.prefs(ctx).edit().putStringSet(KEY, s).apply()
    }

    private fun workName(id: String) = "routine-checkin-$id"
}

/** Commits a pending tap as a done Check-in for today, unless it was undone in the meantime. */
class RoutinesCommitWorker(ctx: Context, p: WorkerParameters) : Worker(ctx, p) {
    override fun doWork(): Result {
        val id = inputData.getString(KEY_ID) ?: return Result.failure()
        if (id !in RoutinesPending.ids(applicationContext)) return Result.success()
        RoutinesPending.set(applicationContext, id, false)
        val out = Core.checkin(Core.dir(applicationContext), id, WidgetData.today(), "done")
        Log.i(TAG, "routine check-in $id: $out")
        Sync.now(applicationContext)
        WidgetRefresh.all(applicationContext)
        return Result.success()
    }

    companion object {
        const val KEY_ID = "routine_id"
    }
}

/** Local-day and JSON helpers shared by the widgets. */
internal object WidgetData {
    fun today(): String = LocalDate.now().toString()

    /** The objects in a JSON array string; an error object such as `{"error":...}` gives none. */
    fun objects(json: String): List<JSONObject> {
        val arr = try {
            JSONArray(json)
        } catch (e: JSONException) {
            JSONArray()
        }
        return (0 until arr.length()).map { arr.getJSONObject(it) }
    }

    /** True when the Routine is due today and not yet ticked (`stats.days` has today as `pending`). */
    fun unchecked(r: JSONObject, today: String): Boolean {
        val days = r.getJSONObject("stats").getJSONArray("days")
        for (i in 0 until days.length()) {
            val d = days.getJSONArray(i)
            if (d.getString(0) == today && d.getString(1) == "pending") return true
        }
        return false
    }

    /** Areas for the config pickers, as (id, name) pairs. */
    fun areas(ctx: Context): List<Pair<String, String>> =
        objects(Core.view_sources(Core.dir(ctx)))
            .filter { it.getString("kind") == "area" }
            .map { it.getString("id") to it.getString("name") }
}
