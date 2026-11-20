// Native capture sheet for the quick-add widget and the list widget's + button.
package dev.orbit.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.Toast
import org.json.JSONObject

class QuickAddActivity : Activity() {
    override fun onCreate(b: Bundle?) {
        super.onCreate(b)
        setContentView(R.layout.activity_quick_add)
        val field = findViewById<EditText>(R.id.quick_text)
        val prefill = intent?.getStringExtra(EXTRA_PREFILL).orEmpty()
        if (prefill.isNotEmpty()) field.setText("$prefill ")
        field.setSelection(field.text.length)
        findViewById<Button>(R.id.quick_submit).setOnClickListener { submit(field.text.toString()) }
    }

    private fun submit(raw: String) {
        val text = raw.trim()
        if (text.isEmpty()) { finish(); return }
        val ctx = applicationContext
        // Rust may hold the connection for a running sync, so keep the main thread out of it.
        Thread {
            val r = JSONObject(Core.capture(Core.dir(ctx), text))
            Sync.now(ctx)
            WidgetRefresh.all(ctx)
            runOnUiThread {
                if (r.has("error")) Toast.makeText(this, R.string.quick_add_failed, Toast.LENGTH_SHORT).show()
                finish()
            }
        }.start()
    }

    companion object {
        const val EXTRA_PREFILL = "prefill"

        /** Opens the sheet; `prefill` is the source's quick-add text such as `@Home` or `#errand`, or empty. */
        fun intent(ctx: Context, prefill: String): Intent =
            Intent(ctx, QuickAddActivity::class.java).putExtra(EXTRA_PREFILL, prefill)
    }
}
