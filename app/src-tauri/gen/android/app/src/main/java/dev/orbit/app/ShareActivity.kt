package dev.orbit.app

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import org.json.JSONObject

/** ACTION_SEND text target: queues an Inbox task in Rust, offline, with a transparent window that closes at once. */
class ShareActivity : Activity() {
    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        val text = intent?.takeIf { it.action == Intent.ACTION_SEND }?.getStringExtra(Intent.EXTRA_TEXT)
        if (text.isNullOrBlank()) { finish(); return }
        // Rust may hold the connection for a running sync, so keep the main thread out of it.
        Thread {
            val r = JSONObject(Core.capture(Core.dir(this), text))
            Log.i(TAG, "capture $r")
            Sync.now(this)
            WidgetRefresh.all(applicationContext)
            runOnUiThread {
                // The window is transparent, so say what happened before it closes.
                Toast.makeText(this, if (r.has("error")) R.string.share_failed else R.string.share_added, Toast.LENGTH_SHORT).show()
                finish()
            }
        }.start()
    }
}
