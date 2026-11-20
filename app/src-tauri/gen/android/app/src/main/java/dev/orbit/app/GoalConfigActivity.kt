// #71 config for the Goal widget: pick all Goals, one Area's Goals, or one Goal as a card. Stored per widget id.
package dev.orbit.app

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import android.widget.ArrayAdapter
import android.widget.ListView

class GoalConfigActivity : Activity() {
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        setResult(RESULT_CANCELED)
        val widget = intent?.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (widget == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        // Each option is (label, mode, area, goal id).
        val options = mutableListOf(Triple(getString(R.string.goal_config_all), "list" to "", ""))
        val dir = Core.dir(this)
        val today = WidgetData.today()
        for ((id, name) in WidgetData.areas(this)) {
            options.add(Triple(getString(R.string.goal_config_area, name), "list" to id, ""))
        }
        for (g in WidgetData.objects(Core.goals(dir, today, ""))) {
            options.add(Triple(getString(R.string.goal_config_card, g.getString("name")), "card" to "", g.getString("id")))
        }
        val list = ListView(this)
        list.adapter = ArrayAdapter(this, android.R.layout.simple_list_item_1, options.map { it.first })
        list.setOnItemClickListener { _, _, pos, _ ->
            val (_, modeArea, goalId) = options[pos]
            val (mode, area) = modeArea
            GoalWidgetProvider.prefs(this).edit()
                .putString("mode_$widget", mode)
                .putString("area_$widget", area)
                .putString("goal_$widget", goalId)
                .apply()
            GoalWidgetProvider.render(this, AppWidgetManager.getInstance(this), intArrayOf(widget))
            setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widget))
            finish()
        }
        setContentView(list)
    }
}
