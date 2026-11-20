// Keeps home-screen widgets in step with the replica: after each sync, and every 15 minutes.
package dev.orbit.app

import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import androidx.work.*
import java.util.concurrent.TimeUnit

object WidgetRefresh {
    /** The AppWidgetProvider classes to refresh. Each widget ticket adds its own. */
    val providers: List<Class<*>> = listOf(QuickAddWidgetProvider::class.java, ListWidgetProvider::class.java)

    /** Asks every placed widget to redraw from the replica. */
    fun all(ctx: Context) {
        val mgr = AppWidgetManager.getInstance(ctx)
        for (p in providers) {
            val ids = mgr.getAppWidgetIds(ComponentName(ctx, p))
            if (ids.isEmpty()) continue
            ctx.sendBroadcast(
                Intent(ctx, p)
                    .setAction(AppWidgetManager.ACTION_APPWIDGET_UPDATE)
                    .putExtra(AppWidgetManager.EXTRA_APPWIDGET_IDS, ids)
            )
        }
    }

    /** Refresh the widgets every 15 minutes, the shortest period WorkManager allows. */
    fun schedule(ctx: Context) {
        val req = PeriodicWorkRequestBuilder<WidgetWorker>(15, TimeUnit.MINUTES).build()
        WorkManager.getInstance(ctx).enqueueUniquePeriodicWork("widgets-periodic", ExistingPeriodicWorkPolicy.KEEP, req)
    }
}

class WidgetWorker(ctx: Context, p: WorkerParameters) : Worker(ctx, p) {
    override fun doWork(): Result {
        WidgetRefresh.all(applicationContext)
        return Result.success()
    }
}
