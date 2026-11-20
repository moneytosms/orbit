// Shared shell of the #71 widget config screens: a plain list, one tap saves the choice and finishes.
package dev.orbit.app

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import android.view.WindowInsets
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
        val list = ListView(this)
        list.adapter = ArrayAdapter(this, android.R.layout.simple_list_item_1, labels())
        list.setOnItemClickListener { _, _, pos, _ ->
            pick(widget, pos)
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
