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

// DPI 变化兜底监听
let initialDPR = window.devicePixelRatio
let layoutVersion = 0

window.addEventListener('resize', () => {
  const currentDPR = window.devicePixelRatio
  if (currentDPR !== initialDPR) {
    console.log(`[DPI Change] ${initialDPR} -> ${currentDPR}`)
    initialDPR = currentDPR
    layoutVersion++
    
    // 触发全局 reflow
    window.dispatchEvent(new CustomEvent('dpi-changed', { 
      detail: { 
        dpr: currentDPR, 
        layoutVersion 
      } 
    }))
    
    // 强制重新计算布局
    requestAnimationFrame(() => {
      document.body.style.display = 'none'
      document.body.offsetHeight // 触发 reflow
      document.body.style.display = ''
    })
  }
})

