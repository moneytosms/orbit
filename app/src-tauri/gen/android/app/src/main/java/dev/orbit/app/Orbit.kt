// Native side that runs with no window: Keystore token store, JNI into the Rust replica, WorkManager sync.
package dev.orbit.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import android.util.Log
import androidx.work.*
import org.json.JSONObject
import java.io.File
import java.security.KeyStore
import java.util.concurrent.TimeUnit
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

const val TAG = "Orbit"

/** Rust owns the replica; this is the only code that names its JNI symbols (see replica.rs). */
object Core {
    init { System.loadLibrary("orbit_app_lib") }
    @JvmStatic external fun capture(dir: String, text: String): String
    @JvmStatic external fun sync(dir: String, server: String, device: String, token: String): String
    @JvmStatic external fun reminders(dir: String, nowLocal: String): String
    @JvmStatic external fun view(dir: String, viewId: String, today: String): String
    @JvmStatic external fun view_sources(dir: String): String
    @JvmStatic external fun whats_next(dir: String, today: String, nowTime: String): String
    @JvmStatic external fun routines(dir: String, today: String, area: String): String
    @JvmStatic external fun goals(dir: String, today: String, area: String): String
    @JvmStatic external fun complete(dir: String, taskId: String, today: String): String
    @JvmStatic external fun checkin(dir: String, routineId: String, date: String, state: String): String
    @JvmStatic external fun checkin_clear(dir: String, routineId: String, date: String): String
    @JvmStatic external fun reminder_snooze(dir: String, taskId: String, untilLocal: String): String
    @JvmStatic external fun enqueue(dir: String, opJson: String): String

    fun dir(ctx: Context) = File(ctx.filesDir, "orbit").path
}

class Auth(val server: String, val device: String, val token: String)

/** The Device token is AES-GCM encrypted with a non-exportable Android Keystore key. */
object Store {
    private const val ALIAS = "orbit-token"
    private fun prefs(ctx: Context) = ctx.getSharedPreferences("orbit", Context.MODE_PRIVATE)

    private fun key(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply { init(spec) }.generateKey()
    }

    fun server(ctx: Context): String? = prefs(ctx).getString("server", null)
    fun setServer(ctx: Context, server: String) = prefs(ctx).edit().putString("server", server).apply()

    fun save(ctx: Context, server: String, device: String, token: String) {
        val c = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val blob = c.iv + c.doFinal(token.toByteArray())
        prefs(ctx).edit().putString("server", server).putString("device", device)
            .putString("token", Base64.encodeToString(blob, Base64.NO_WRAP)).commit()
    }

    fun load(ctx: Context): Auth? {
        val p = prefs(ctx)
        val server = p.getString("server", null) ?: return null
        val device = p.getString("device", null) ?: return null
        val blob = Base64.decode(p.getString("token", null) ?: return null, Base64.NO_WRAP)
        return try {
            val c = Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, blob, 0, 12))
            Auth(server, device, String(c.doFinal(blob, 12, blob.size - 12)))
        } catch (e: Exception) {
            Log.w(TAG, "token unreadable, signing out: $e"); clearToken(ctx); null
        }
    }

    fun clearToken(ctx: Context) = prefs(ctx).edit().remove("token").remove("device").commit()
}

object Sync {
    private val online = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()

    /** Send the outbox as soon as there is a connection; WorkManager keeps it across kill and reboot. */
    fun now(ctx: Context) {
        val req = OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(online).build()
        WorkManager.getInstance(ctx).enqueueUniqueWork("sync", ExistingWorkPolicy.APPEND_OR_REPLACE, req)
    }

    fun schedule(ctx: Context) {
        val req = PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES).setConstraints(online).build()
        WorkManager.getInstance(ctx).enqueueUniquePeriodicWork("sync-periodic", ExistingPeriodicWorkPolicy.KEEP, req)
    }
}

class SyncWorker(ctx: Context, p: WorkerParameters) : Worker(ctx, p) {
    override fun doWork(): Result {
        val auth = Store.load(applicationContext) ?: return Result.success() // signed out: captures wait
        val r = JSONObject(Core.sync(Core.dir(applicationContext), auth.server, auth.device, auth.token))
        Log.i(TAG, "sync ${r.toString().replace(auth.token, "***")}")
        if (r.optBoolean("signed_out")) { Store.clearToken(applicationContext); return Result.success() }
        if (r.has("error")) return Result.retry()
        WidgetRefresh.all(applicationContext)
        return Result.success()
    }
}
