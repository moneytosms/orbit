package dev.orbit.app

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import org.json.JSONObject

/** ACTION_SEND text target: queues an Inbox task in Rust, offline, with a transparent window that closes at once. */
class ShareActivity : Activity() {
    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        val text = intent?.takeIf { it.action == Intent.ACTION_SEND }?.getStringExtra(Intent.EXTRA_TEXT)
        if (text.isNullOrBlank()) { finish(); return }
        // Rust may hold the connection for a running sync, so keep the main thread out of it.
        Thread {
            Log.i(TAG, "capture ${JSONObject(Core.capture(Core.dir(this), text))}")
            Sync.now(this)
            runOnUiThread { finish() }
        }.start()
    }
}
