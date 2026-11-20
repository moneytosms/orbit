// #71 config for the Routines widget: pick "all areas" or one Area, stored per widget id.
package dev.orbit.app

import android.appwidget.AppWidgetManager

class RoutinesConfigActivity : PickerConfigActivity() {
    private val areas by lazy { WidgetData.areas(this) }

    override fun labels() = listOf(getString(R.string.routines_config_all)) + areas.map { it.second }

    override fun pick(widget: Int, pos: Int) {
        val area = if (pos == 0) "" else areas[pos - 1].first
        RoutinesWidgetProvider.prefs(this).edit().putString("area_$widget", area).apply()
        RoutinesWidgetProvider.render(this, AppWidgetManager.getInstance(this), intArrayOf(widget))
    }
}
