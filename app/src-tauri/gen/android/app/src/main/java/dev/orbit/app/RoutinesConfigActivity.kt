// #71 config for the Routines widget: pick "all areas" or one Area, stored per widget id.
package dev.orbit.app

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import android.widget.ArrayAdapter
import android.view.WindowInsets
import android.widget.ListView

class RoutinesConfigActivity : Activity() {
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        setResult(RESULT_CANCELED)
        val widget = intent?.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (widget == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        val areas = WidgetData.areas(this)
        val labels = listOf(getString(R.string.routines_config_all)) + areas.map { it.second }
        val values = listOf("") + areas.map { it.first }
        val list = ListView(this)
        list.adapter = ArrayAdapter(this, android.R.layout.simple_list_item_1, labels)
        list.setOnItemClickListener { _, _, pos, _ ->
            RoutinesWidgetProvider.prefs(this).edit().putString("area_$widget", values[pos]).apply()
            RoutinesWidgetProvider.render(this, AppWidgetManager.getInstance(this), intArrayOf(widget))
            setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widget))
            finish()
        }
        // Android 15 draws edge to edge: keep the first row out from under the status bar.
        list.clipToPadding = false
        list.setOnApplyWindowInsetsListener { v, insets ->
            val bars = insets.getInsets(WindowInsets.Type.systemBars())
            v.setPadding(0, bars.top, 0, bars.bottom)
            insets
        }
        setContentView(list)
    }
}
