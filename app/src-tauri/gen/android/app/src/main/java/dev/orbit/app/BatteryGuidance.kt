// One-time guidance: OEM battery savers can delay or kill reminder alarms.
package dev.orbit.app

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings

object BatteryGuidance {
    private const val PREFS = "orbit-ui"
    private const val SEEN = "battery_guidance_seen"

    fun maybeShow(activity: Activity) {
        val prefs = activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (prefs.getBoolean(SEEN, false)) return
        prefs.edit().putBoolean(SEEN, true).apply()
        AlertDialog.Builder(activity, R.style.Theme_Orbit_Alert)
            .setTitle(R.string.battery_title)
            .setMessage(R.string.battery_message)
            .setPositiveButton(R.string.battery_open) { _, _ ->
                val uri = Uri.fromParts("package", activity.packageName, null)
                activity.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, uri))
            }
            .setNegativeButton(R.string.battery_later, null)
            .show()
    }
}
