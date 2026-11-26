// Source picker for one list widget. Opened from the widget's header tap or on placement.
package dev.orbit.app

import android.appwidget.AppWidgetManager
import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.widget.ArrayAdapter
import android.widget.ListView
import org.json.JSONObject

class ListWidgetConfigActivity : Activity() {
    private var widgetId = AppWidgetManager.INVALID_APPWIDGET_ID

    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        setContentView(R.layout.activity_list_widget_config)
        widgetId = intent?.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        setResult(RESULT_CANCELED, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
        if (widgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        findViewById<android.widget.TextView>(R.id.config_title).text = title
        val list = findViewById<ListView>(R.id.config_list)
        list.clipToPadding = false
        list.padForSystemBars()
        barIconsFollowTheme()
        // Core JNI can wait on the connection during a sync, so the list loads off the main thread.
        Thread {
            val all = ListWidget.sources(applicationContext)
            val items = (0 until (all?.length() ?: 0)).map { all!!.getJSONObject(it) }
            runOnUiThread {
                list.adapter = ArrayAdapter(this, R.layout.item_picker, items.map { it.optString("name") })
                list.setOnItemClickListener { _, _, pos, _ -> pick(items[pos]) }
            }
        }.start()
    }

    private fun pick(src: JSONObject) {
        ListWidget.save(applicationContext, widgetId, src)
        val ctx = applicationContext
        Thread {
            ListWidgetProvider.render(ctx, AppWidgetManager.getInstance(ctx), widgetId)
            runOnUiThread {
                setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
                finish()
            }
        }.start()
    }
}
