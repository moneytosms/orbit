// #71 Goal widget: one Goal as a card, or a list of Goals (all, or one Area). View only: a tap opens Orbit.
package dev.orbit.app

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import kotlin.math.roundToInt

class GoalWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) = render(ctx, mgr, ids)

    override fun onDeleted(ctx: Context, ids: IntArray) {
        val edit = prefs(ctx).edit()
        for (id in ids) edit.remove("mode_$id").remove("area_$id").remove("goal_$id")
        edit.apply()
    }

    companion object {
        const val EXTRA_OPEN = "orbit_open"
        private const val PREFS = "goal_widgets"

        fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

        /** Redraws the given widgets from the replica. The config activity calls this after a choice. */
        fun render(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
            val dir = Core.dir(ctx)
            val today = WidgetData.today()
            val p = prefs(ctx)
            // Row tap: the template opens Orbit, the row's fill-in intent carries which Goal.
            val template = PendingIntent.getActivity(
                ctx, 0, Intent(ctx, MainActivity::class.java),
                PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            for (widget in ids) {
                val views = RemoteViews(ctx.packageName, R.layout.widget_goal)
                if (p.getString("mode_$widget", "list") == "card") {
                    val goalId = p.getString("goal_$widget", "") ?: ""
                    val goal = WidgetData.objects(Core.goals(dir, today, ""))
                        .firstOrNull { it.getString("id") == goalId }
                    // The card has no "Goals" header, and neither does the empty message.
                    views.setViewVisibility(R.id.header, View.GONE)
                    if (goal == null) {
                        views.setTextViewText(R.id.empty, ctx.getString(R.string.goal_missing))
                        views.setViewVisibility(R.id.empty, View.VISIBLE)
                    } else {
                        val pct = percent(goal)
                        views.setTextViewText(R.id.card_title, goal.getString("name"))
                        views.setProgressBar(R.id.card_bar, 100, pct, false)
                        views.setTextViewText(R.id.card_percent, ctx.getString(R.string.goal_percent, pct))
                        views.setTextViewText(R.id.card_numbers, counts(goal) ?: "")
                        views.setViewVisibility(R.id.card_chip, visible(goal.optBoolean("behind")))
                        views.setViewVisibility(R.id.card_target, visible(!goal.isNull("target_date")))
                        if (!goal.isNull("target_date")) {
                            views.setTextViewText(
                                R.id.card_target, ctx.getString(R.string.goal_target, goal.getString("target_date"))
                            )
                        }
                        views.setOnClickPendingIntent(R.id.card, openGoal(ctx, goalId))
                        views.setViewVisibility(R.id.card, View.VISIBLE)
                    }
                } else {
                    val area = p.getString("area_$widget", "") ?: ""
                    val goals = WidgetData.objects(Core.goals(dir, today, area))
                    val items = RemoteViews.RemoteCollectionItems.Builder().setHasStableIds(true).setViewTypeCount(1)
                    for (g in goals) {
                        val gid = g.getString("id")
                        val pct = percent(g)
                        val row = RemoteViews(ctx.packageName, R.layout.widget_goal_row)
                        row.setTextViewText(R.id.name, g.getString("name"))
                        row.setProgressBar(R.id.bar, 100, pct, false)
                        row.setTextViewText(R.id.percent, ctx.getString(R.string.goal_percent, pct))
                        row.setTextViewText(R.id.counts, counts(g) ?: "")
                        row.setViewVisibility(R.id.counts, visible(!g.isNull("total")))
                        row.setViewVisibility(R.id.chip, visible(g.optBoolean("behind")))
                        row.setOnClickFillInIntent(R.id.row, Intent().putExtra(EXTRA_OPEN, "goal:$gid"))
                        items.addItem(gid.hashCode().toLong(), row)
                    }
                    views.setRemoteAdapter(R.id.list, items.build())
                    views.setPendingIntentTemplate(R.id.list, template)
                    views.setViewVisibility(R.id.list, visible(goals.isNotEmpty()))
                    views.setTextViewText(R.id.empty, ctx.getString(R.string.goal_none))
                    views.setViewVisibility(R.id.empty, visible(goals.isEmpty()))
                }
                mgr.updateAppWidget(widget, views)
            }
        }

        private fun openGoal(ctx: Context, goalId: String) = PendingIntent.getActivity(
            ctx, goalId.hashCode(),
            Intent(ctx, MainActivity::class.java).putExtra(EXTRA_OPEN, "goal:$goalId"),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        /** "3/10" for a counted or target Goal, null for a Manual one. */
        private fun counts(g: JSONObject): String? {
            if (g.isNull("total")) return null
            fun n(d: Double) = if (d == Math.floor(d)) d.toLong().toString() else "%.1f".format(d)
            return "${n(g.getDouble("done"))}/${n(g.getDouble("total"))}"
        }

        private fun percent(g: JSONObject): Int = (g.optDouble("progress", 0.0) * 100).roundToInt().coerceIn(0, 100)

        private fun visible(on: Boolean): Int = if (on) View.VISIBLE else View.GONE
    }
}
