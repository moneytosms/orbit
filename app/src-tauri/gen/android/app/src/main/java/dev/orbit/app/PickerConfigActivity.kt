// Shared shell of the #71 widget config screens: a plain list, one tap saves the choice and finishes.
package dev.orbit.app

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import android.widget.ArrayAdapter
import android.widget.ListView

abstract class PickerConfigActivity : Activity() {
    /** One label per choice, in list order. */
    abstract fun labels(): List<String>

    /** Stores choice `pos` for `widget` and repaints it. */
    abstract fun pick(widget: Int, pos: Int)

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        setResult(RESULT_CANCELED)
        val widget = intent?.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (widget == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        setContentView(R.layout.activity_list_widget_config)
        findViewById<android.widget.TextView>(R.id.config_title).text = title
        val list = findViewById<ListView>(R.id.config_list)
        list.adapter = ArrayAdapter(this, R.layout.item_picker, labels())
        list.setOnItemClickListener { _, _, pos, _ ->
            pick(widget, pos)
            setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widget))
            finish()
        }
        // Android 15 draws edge to edge: keep the first row out from under the status bar.
        list.clipToPadding = false
        list.padForSystemBars()
        barIconsFollowTheme() // needs the decor view, so after setContentView
    }
}
