package dev.orbit.app

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.util.Log
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

object ShellUpdate {
    private const val RELEASES = "https://github.com/moneytosms/orbit/releases/download"

    fun check(activity: Activity) {
        val server = Store.server(activity) ?: return
        Thread {
            val minimum = try {
                val connection = URL("${server.trimEnd('/')}/version").openConnection() as HttpURLConnection
                connection.connectTimeout = 5_000
                connection.readTimeout = 5_000
                connection.setRequestProperty("Cache-Control", "no-cache")
                connection.inputStream.bufferedReader().use { JSONObject(it.readText()).optString("min_shell_version") }
                    .also { connection.disconnect() }
            } catch (e: Exception) {
                Log.i(TAG, "native version check unavailable: ${e.javaClass.simpleName}")
                return@Thread
            }
            val installed = activity.packageManager.getPackageInfo(activity.packageName, 0).versionName ?: return@Thread
            if (compareVersions(installed, minimum) >= 0 || compareVersions(minimum, "0.0.0") <= 0) return@Thread

            activity.runOnUiThread {
                val link = "$RELEASES/v$minimum/Orbit-$minimum-android.apk"
                AlertDialog.Builder(activity)
                    .setTitle("Update the app")
                    .setMessage("This server requires Orbit $minimum. This app is $installed. Download the Android APK to continue.")
                    .setPositiveButton("Download APK") { _, _ ->
                        activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link)))
                    }
                    .setCancelable(false)
                    .show()
                    .setCanceledOnTouchOutside(false)
            }
        }.start()
    }

    private fun compareVersions(left: String, right: String): Int {
        val a = left.substringBefore('-').removePrefix("v").split('.').map { it.toIntOrNull() ?: 0 }
        val b = right.substringBefore('-').removePrefix("v").split('.').map { it.toIntOrNull() ?: 0 }
        for (i in 0 until maxOf(a.size, b.size)) {
            val difference = (a.getOrElse(i) { 0 }).compareTo(b.getOrElse(i) { 0 })
            if (difference != 0) return difference
        }
        return 0
    }
}
