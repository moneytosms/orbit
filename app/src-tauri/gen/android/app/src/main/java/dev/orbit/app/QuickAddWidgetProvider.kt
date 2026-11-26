// A 1x1 to 5x1 home-screen button: a tap opens the native capture sheet.
package dev.orbit.app

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.widget.RemoteViews

class QuickAddWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        for (id in ids) {
            val views = RemoteViews(ctx.packageName, R.layout.widget_quick_add)
            val open = Link.capture(ctx, "")
            views.setOnClickPendingIntent(R.id.quick_root, ListWidget.activity(ctx, id, open))
            mgr.updateAppWidget(id, views)
        }
    }
}
