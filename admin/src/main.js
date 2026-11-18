import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { setUnauthorizedHandler } from './api'
import { forgetSession } from './auth'
import './style.css'

setUnauthorizedHandler(() => {
  const current = router.currentRoute.value
  if (current.name === 'login') return
  forgetSession()
  router.replace({
    name: 'login',
    query: { next: current.fullPath },
  })
})

const app = createApp(App)

app.use(router)

app.mount('#app')
