import 'virtual:catppuccin.css'
import './styles/main.css'

import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from './App.vue'
import router from './router'
import { useThemeStore } from './stores/theme'

const app = createApp(App)
app.use(createPinia())
useThemeStore().init()
app.use(router)
app.mount('#app')
