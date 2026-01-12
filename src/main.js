import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import App from './App.vue'

// 全局禁用右键菜单
document.addEventListener('contextmenu', (e) => {
  e.preventDefault()
  return false
})

// 全局禁用文本选择（可选，如果不需要可以选择性移除）
document.addEventListener('selectstart', (e) => {
  e.preventDefault()
  return false
})

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.mount('#app')

