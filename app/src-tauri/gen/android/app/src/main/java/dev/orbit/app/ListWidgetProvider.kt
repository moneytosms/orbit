// A resizable task list. Rows come from Core JNI; circle taps complete after a 2s undo window.
package dev.orbit.app

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.text.SpannableString
import android.text.style.StrikethroughSpan
import android.util.Log
import android.view.View
import android.widget.RemoteViews
import androidx.work.*
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.util.concurrent.TimeUnit

/** One line of a list: a day header, or a Task row. */
class ListEntry(val header: String?, val row: JSONObject?)

/** Constants, saved choices and the data load shared by the list widget, its config screen and its worker. */
object ListWidget {
    const val ACTION_ITEM = "dev.orbit.app.LIST_ITEM"
    const val EXTRA_ACTION = "item_action"
    const val EXTRA_TASK = "task_id"
    const val EXTRA_LINK = "orbit_link"
    const val ACTION_COMPLETE = "complete"
    const val ACTION_OPEN = "open"
    const val UNDO_MS = 2000L

    val DAY: DateTimeFormatter = DateTimeFormatter.ofPattern("EEE d MMM")
    private val HOUR_MIN: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")

    private fun prefs(ctx: Context) = ctx.getSharedPreferences("widgets", Context.MODE_PRIVATE)

    /** A PendingIntent for a widget's header or + button; one request code per widget and purpose. */
    fun activity(ctx: Context, code: Int, intent: Intent): PendingIntent =
        PendingIntent.getActivity(ctx, code, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)

    /** The `view_sources` entries, or null when Core answers with an error. */
    fun sources(ctx: Context): JSONArray? = array(Core.view_sources(Core.dir(ctx)))

    /** The source a widget shows: its saved choice, else the Inbox view. */
    fun source(ctx: Context, widgetId: Int): JSONObject {
        prefs(ctx).getString("src:$widgetId", null)?.let { return JSONObject(it) }
        val all = sources(ctx)?.let { a -> (0 until a.length()).map { a.getJSONObject(it) } }.orEmpty()
        return all.firstOrNull { it.optString("kind") == "view" && it.optString("name") == "Inbox" }
            ?: all.firstOrNull()
            ?: JSONObject().put("kind", "view").put("id", "").put("name", "Inbox").put("prefill", "")
    }

    fun save(ctx: Context, widgetId: Int, src: JSONObject) {
        prefs(ctx).edit().putString("src:$widgetId", src.toString()).apply()
    }

    fun forget(ctx: Context, widgetIds: IntArray) {
        val edit = prefs(ctx).edit()
        for (id in widgetIds) edit.remove("src:$id")
        edit.apply()
    }

    /** The commit deadline of a Task completed but not yet sent to Core; 0 when none is pending. */
    fun pendingDeadline(ctx: Context, taskId: String): Long = prefs(ctx).getLong("pending:$taskId", 0L)

    fun setPending(ctx: Context, taskId: String, deadline: Long) {
        prefs(ctx).edit().putLong("pending:$taskId", deadline).apply()
    }

    fun clearPending(ctx: Context, taskId: String) {
        prefs(ctx).edit().remove("pending:$taskId").apply()
    }

    fun scheduleCommit(ctx: Context, taskId: String, deadline: Long) {
        val req = OneTimeWorkRequestBuilder<CompleteWorker>()
            .setInitialDelay(UNDO_MS, TimeUnit.MILLISECONDS)
            .setInputData(workDataOf("task" to taskId, "deadline" to deadline))
            .build()
        WorkManager.getInstance(ctx).enqueueUniqueWork("complete-$taskId", ExistingWorkPolicy.REPLACE, req)
    }

    fun cancelCommit(ctx: Context, taskId: String) {
        WorkManager.getInstance(ctx).cancelUniqueWork("complete-$taskId")
    }

    /** The entries a source shows today, with day headers where Core grouped them. */
    fun load(ctx: Context, src: JSONObject): List<ListEntry> {
        val dir = Core.dir(ctx)
        val today = LocalDate.now().toString()
        val id = src.optString("id")
        return when (val kind = src.optString("kind")) {
            "view" -> groups(Core.view(dir, id, today))
            "next" -> next(Core.whats_next(dir, today, LocalDateTime.now().format(HOUR_MIN)))
            "area", "project", "label" -> groups(Core.list(dir, kind, id, today))
            else -> emptyList()
        }
    }

    private fun groups(text: String): List<ListEntry> {
        val arr = array(text) ?: return emptyList()
        val out = ArrayList<ListEntry>()
        for (i in 0 until arr.length()) {
            val g = arr.getJSONObject(i)
            val rows = g.getJSONArray("rows")
            if (rows.length() == 0) continue
            val key = g.optString("key")
            if (key.isNotEmpty()) out.add(ListEntry(header(key), null))
            for (j in 0 until rows.length()) out.add(ListEntry(null, rows.getJSONObject(j)))
        }
        return out
    }

    /** What's next items reshaped as Task rows; the breadcrumb stands in for the project. */
    private fun next(text: String): List<ListEntry> {
        val arr = array(text) ?: return emptyList()
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            val bc = o.optJSONArray("breadcrumb")
            val path = bc?.let { b -> (0 until b.length()).joinToString(" › ") { b.optString(it) } }.orEmpty()
            val row = JSONObject()
                .put("id", o.optString("id"))
                .put("title", o.optString("title"))
                .put("priority", 4)
                .put("project", path)
            ListEntry(null, row)
        }
    }

    private fun header(key: String): String = try {
        LocalDate.parse(key).format(DAY)
    } catch (e: DateTimeParseException) {
        key // "Overdue" and "No date"
    }

    private fun array(text: String): JSONArray? = try {
        JSONArray(text)
    } catch (e: JSONException) {
        Log.w(TAG, "widget data: $text")
        null
    }
}

class ListWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        // Core JNI reads the replica, so draw off the main thread; goAsync keeps the receiver alive.
        val pending = goAsync()
        Thread {
            for (id in ids) render(ctx, mgr, id)
            pending.finish()
        }.start()
    }

    override fun onDeleted(ctx: Context, ids: IntArray) {
        ListWidget.forget(ctx, ids)
    }

    override fun onReceive(ctx: Context, intent: Intent) {
        if (intent.action != ListWidget.ACTION_ITEM) {
            super.onReceive(ctx, intent)
            return
        }
        val taskId = intent.getStringExtra(ListWidget.EXTRA_TASK) ?: return
        when (intent.getStringExtra(ListWidget.EXTRA_ACTION)) {
            ListWidget.ACTION_COMPLETE -> toggleComplete(ctx, taskId)
            ListWidget.ACTION_OPEN -> {
                ctx.startActivity(
                    Intent(ctx, MainActivity::class.java)
                        .putExtra(ListWidget.EXTRA_LINK, "orbit://task/$taskId")
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                )
            }
        }
    }

    /** First tap marks the row pending; a tap within the undo window cancels it. */
    private fun toggleComplete(ctx: Context, taskId: String) {
        if (ListWidget.pendingDeadline(ctx, taskId) > 0) {
            ListWidget.clearPending(ctx, taskId)
            ListWidget.cancelCommit(ctx, taskId)
        } else {
            val deadline = System.currentTimeMillis() + ListWidget.UNDO_MS
            ListWidget.setPending(ctx, taskId, deadline)
            ListWidget.scheduleCommit(ctx, taskId, deadline)
        }
        WidgetRefresh.all(ctx)
    }

    companion object {
        /** Redraws one list widget from the replica. Runs Core JNI, so call it off the main thread. */
        fun render(ctx: Context, mgr: AppWidgetManager, id: Int) {
            val src = ListWidget.source(ctx, id)
            val entries = ListWidget.load(ctx, src)
            val views = RemoteViews(ctx.packageName, R.layout.widget_list)
            views.setTextViewText(R.id.list_title, src.optString("name"))
            val config = Intent(ctx, ListWidgetConfigActivity::class.java)
                .putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, id)
            views.setOnClickPendingIntent(R.id.list_title, ListWidget.activity(ctx, 200000 + id, config))
            val add = QuickAddActivity.intent(ctx, src.optString("prefill"))
            views.setOnClickPendingIntent(R.id.list_add, ListWidget.activity(ctx, 100000 + id, add))

            val items = RemoteViews.RemoteCollectionItems.Builder().setViewTypeCount(2)
            for ((i, e) in entries.withIndex()) items.addItem(i.toLong(), itemViews(ctx, e))
            views.setRemoteAdapter(R.id.list_items, items.build())
            views.setEmptyView(R.id.list_items, R.id.list_empty)

            // Row clicks are fill-in intents on this template: the circle completes, the row opens.
            val template = PendingIntent.getBroadcast(
                ctx,
                300000 + id,
                Intent(ctx, ListWidgetProvider::class.java).setAction(ListWidget.ACTION_ITEM),
                PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            views.setPendingIntentTemplate(R.id.list_items, template)
            mgr.updateAppWidget(id, views)
        }

        private fun itemViews(ctx: Context, e: ListEntry): RemoteViews {
            e.header?.let { h ->
                return RemoteViews(ctx.packageName, R.layout.widget_list_header).apply {
                    setTextViewText(R.id.header_text, h)
                }
            }
            val row = e.row ?: return RemoteViews(ctx.packageName, R.layout.widget_list_header)
            val taskId = row.optString("id")
            val done = ListWidget.pendingDeadline(ctx, taskId) > 0 || row.optBoolean("completed")
            val title = row.optString("title")
            val views = RemoteViews(ctx.packageName, R.layout.widget_list_row)
            views.setImageViewResource(R.id.row_circle, if (done) R.drawable.filled_circle else R.drawable.ring)
            views.setInt(R.id.row_circle, "setColorFilter", priorityColor(row.optInt("priority", 4)))
            views.setTextViewText(R.id.row_title, if (done) strike(title) else title)

            val due = dueLabel(row)
            views.setViewVisibility(R.id.row_due, if (due == null) View.GONE else View.VISIBLE)
            if (due != null) views.setTextViewText(R.id.row_due, due)

            val glyphs = listOfNotNull(
                if (row.optBoolean("recurring")) "↻" else null,
                if (row.optBoolean("has_reminder")) "🔔" else null,
                row.optInt("sub_total").takeIf { it > 0 }?.let { "${row.optInt("sub_done")}/$it" }
            ).joinToString(" ")
            views.setViewVisibility(R.id.row_glyphs, if (glyphs.isEmpty()) View.GONE else View.VISIBLE)
            views.setTextViewText(R.id.row_glyphs, glyphs)

            val area = row.str("area")
            val meta = listOfNotNull(area, row.str("project")).joinToString(" · ")
            views.setViewVisibility(R.id.row_meta, if (meta.isEmpty()) View.GONE else View.VISIBLE)
            views.setTextViewText(R.id.row_meta, meta)
            views.setViewVisibility(R.id.row_dot, if (area == null) View.GONE else View.VISIBLE)
            // An Area's own colour (`#rrggbb`, from #72) when it has one.
            val dot = row.str("area_color")?.let { runCatching { Color.parseColor(it) }.getOrNull() }
            views.setInt(R.id.row_dot, "setColorFilter", dot ?: accent(ctx))

            views.setOnClickFillInIntent(R.id.row_circle, fill(ListWidget.ACTION_COMPLETE, taskId))
            views.setOnClickFillInIntent(R.id.row_root, fill(ListWidget.ACTION_OPEN, taskId))
            return views
        }

        private fun fill(action: String, taskId: String): Intent =
            Intent().putExtra(ListWidget.EXTRA_ACTION, action).putExtra(ListWidget.EXTRA_TASK, taskId)

        private fun priorityColor(priority: Int): Int = when (priority) {
            1 -> Color.parseColor("#D32F2F")
            2 -> Color.parseColor("#F57C00")
            3 -> Color.parseColor("#1976D2")
            else -> Color.parseColor("#9E9E9E")
        }

        /** The system accent on Android 12+, a static blue below that. */
        private fun accent(ctx: Context): Int =
            if (Build.VERSION.SDK_INT >= 31) ctx.getColor(android.R.color.system_accent1_600) else Color.parseColor("#3949AB")

        private fun strike(s: String): CharSequence =
            SpannableString(s).apply { setSpan(StrikethroughSpan(), 0, length, SpannableString.SPAN_INCLUSIVE_EXCLUSIVE) }

        /** Today, Tomorrow or the date, then the time; null when the Task has no date. */
        private fun dueLabel(row: JSONObject): String? {
            val date = row.str("due") ?: return null
            val d = LocalDate.parse(date)
            val today = LocalDate.now()
            val day = when (d) {
                today -> "Today"
                today.plusDays(1) -> "Tomorrow"
                else -> d.format(ListWidget.DAY)
            }
            return row.str("due_time")?.let { "$day $it" } ?: day
        }

        private fun JSONObject.str(key: String): String? =
            if (isNull(key)) null else optString(key).ifEmpty { null }
    }
}

/** Runs after the undo window: commits the completion if it was not cancelled. */
class CompleteWorker(ctx: Context, p: WorkerParameters) : Worker(ctx, p) {
    override fun doWork(): Result {
        val app = applicationContext
        val taskId = inputData.getString("task") ?: return Result.failure()
        val deadline = inputData.getLong("deadline", 0L)
        if (ListWidget.pendingDeadline(app, taskId) != deadline) return Result.success() // undone
        ListWidget.clearPending(app, taskId)
        val r = JSONObject(Core.complete(Core.dir(app), taskId, LocalDate.now().toString()))
        Log.i(TAG, "widget complete ${r.toString().take(120)}")
        Sync.now(app)
        WidgetRefresh.all(app)
        return Result.success()
    }
}
