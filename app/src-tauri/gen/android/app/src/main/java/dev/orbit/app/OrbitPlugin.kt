package dev.orbit.app

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg class TokenArgs { lateinit var server: String; lateinit var device: String; lateinit var token: String }
@InvokeArg class ServerArgs { lateinit var server: String }

/** Called from Rust (android.rs): token and server storage for the webview's Tauri commands. */
@TauriPlugin
class OrbitPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun setToken(invoke: Invoke) {
        val a = invoke.parseArgs(TokenArgs::class.java)
        Store.save(activity, a.server, a.device, a.token)
        Sync.schedule(activity)
        Sync.now(activity)
        invoke.resolve()
    }

    @Command
    fun hasToken(invoke: Invoke) = invoke.resolve(JSObject().put("has", Store.load(activity) != null))

    @Command
    fun clearToken(invoke: Invoke) { Store.clearToken(activity); invoke.resolve() }

    @Command
    fun setServer(invoke: Invoke) { Store.setServer(activity, invoke.parseArgs(ServerArgs::class.java).server); invoke.resolve() }

    @Command
    fun getServer(invoke: Invoke) = invoke.resolve(JSObject().put("server", Store.server(activity) ?: ""))
}
