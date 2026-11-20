// #71 config for the Goal widget: pick all Goals, one Area's Goals, or one Goal as a card. Stored per widget id.
package dev.orbit.app

import android.appwidget.AppWidgetManager

/** One choice: its label, `list` or `card` mode, the Area to filter by, and the Goal for a card. */
private data class Choice(val label: String, val mode: String, val area: String = "", val goal: String = "")

class GoalConfigActivity : PickerConfigActivity() {
    private val choices by lazy {
        val all = listOf(Choice(getString(R.string.goal_config_all), "list"))
        val byArea = WidgetData.areas(this).map { (id, name) -> Choice(getString(R.string.goal_config_area, name), "list", area = id) }
        val cards = WidgetData.objects(Core.goals(Core.dir(this), WidgetData.today(), ""))
            .map { Choice(getString(R.string.goal_config_card, it.getString("name")), "card", goal = it.getString("id")) }
        all + byArea + cards
    }

    override fun labels() = choices.map { it.label }

    override fun pick(widget: Int, pos: Int) {
        val c = choices[pos]
        GoalWidgetProvider.prefs(this).edit()
            .putString("mode_$widget", c.mode)
            .putString("area_$widget", c.area)
            .putString("goal_$widget", c.goal)
            .apply()
        GoalWidgetProvider.render(this, AppWidgetManager.getInstance(this), intArrayOf(widget))
    }
}
