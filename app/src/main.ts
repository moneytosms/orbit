import { mount } from 'svelte'
import init from '../wasm/orbit_wasm'
import './app.css'
import './lib/theme'
import App from './App.svelte'

await init()
export default mount(App, { target: document.getElementById('app')! })
