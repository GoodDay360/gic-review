import 'virtual:catppuccin.css'
import './styles/main.css'
import 'animate.css'
import 'aos/dist/aos.css'

import AOS from 'aos'
import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from './App.vue'
import { vuetify } from './plugins/vuetify'
import router from './router'
import { useThemeStore } from './stores/theme'

const app = createApp(App)
app.use(createPinia())
app.use(vuetify)
useThemeStore().init()
app.use(router)
app.mount('#app')

// Animate on scroll: elements with `data-aos` fade in once as they scroll into view.
// AOS watches the DOM, so content loaded later (infinite lists, route changes) works too.
AOS.init({
  once: true,
  duration: 550,
  easing: 'ease-out-cubic',
  offset: 40,
  disable: () => window.matchMedia('(prefers-reduced-motion: reduce)').matches,
})
router.afterEach(() => setTimeout(() => AOS.refresh(), 50))
