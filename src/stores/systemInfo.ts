import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'

export interface SystemInfo {
  platform: string
  arch: string
  osName?: string
  osDisplayVersion?: string
  buildNumber?: string
  ubr?: number
  osDisplay?: string
  osVersion?: string // 保持向后兼容
  appVersion: string
  kernelVersion?: string
}

export const useSystemInfoStore = defineStore('systemInfo', {
  state: () => ({
    info: null as SystemInfo | null,
    status: 'idle' as 'idle' | 'loading' | 'success' | 'error',
    error: null as string | null,
    loadedAt: null as number | null,
    inFlight: null as Promise<void> | null,
  }),

  actions: {
    /**
     * 加载系统信息（仅加载一次，已加载则直接返回）
     */
    async loadOnce() {
      // 如果已成功加载，直接返回
      if (this.status === 'success' && this.info) {
        return
      }

      // 如果正在加载中，等待当前请求完成
      if (this.status === 'loading' && this.inFlight) {
        return this.inFlight
      }

      // 开始新的加载
      this.status = 'loading'
      this.error = null

      const loadPromise = (async () => {
        try {
          const info = await invoke<SystemInfo>('get_system_info')
          this.info = info
          this.status = 'success'
          this.loadedAt = Date.now()
          this.error = null
        } catch (err) {
          this.status = 'error'
          this.error = err instanceof Error ? err.message : String(err)
          console.error('获取系统信息失败:', err)
        } finally {
          this.inFlight = null
        }
      })()

      this.inFlight = loadPromise
      return loadPromise
    },

    /**
     * 强制刷新系统信息
     */
    async refresh() {
      // 清空状态，强制重新加载
      this.info = null
      this.status = 'idle'
      this.loadedAt = null
      this.error = null
      this.inFlight = null

      return this.loadOnce()
    },
  },
})
