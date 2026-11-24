import { mount } from 'svelte'
import init from '../wasm/orbit_wasm'
import './app.css'
import './lib/theme'
import App from './App.svelte'

await init()
if (import.meta.env.VITE_DEMO) await (await import('./lib/demo')).startDemo()
export default mount(App, { target: document.getElementById('app')! })
