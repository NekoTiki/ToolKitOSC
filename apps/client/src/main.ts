import './assets/main.css'

import ui from '@nuxt/ui/vue-plugin'
import { createApp } from 'vue'

import App from './App.vue'
import { startParameterLog } from './composables/useParameterLog'
import { router } from './router'

const app = createApp(App)

app.use(router)
app.use(ui)

app.mount('#app')

// Records from launch, so Parameters > Change log already has history when first opened.
startParameterLog()
