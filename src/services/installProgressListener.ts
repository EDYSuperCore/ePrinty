import { listen } from '@tauri-apps/api/event'
import { normalizeProgressEvent } from '../domain/progress/schema'

interface InstallProgressStore {
  applyEvent: (evt: any) => void
  activeJobId: string | null
  setActiveJob: (jobId: string) => void
}

interface CreateListenerOptions {
  store: InstallProgressStore
  openInstallModal: () => void
  shouldSilenceSelfcheck?: (payload: any) => boolean
}

export function createInstallProgressListener(options: CreateListenerOptions) {
  const { store, openInstallModal, shouldSilenceSelfcheck } = options
  let unlistenProgress: (() => void) | null = null

  const isSelfcheck = (payload: any) => {
    if (typeof shouldSilenceSelfcheck === 'function') {
      return shouldSilenceSelfcheck(payload)
    }
    const rawJobId = payload?.jobId || payload?.job_id || ''
    const printerName = payload?.printerName || payload?.printer_name || ''
    return rawJobId === 'selfcheck' || printerName === 'selfcheck'
  }

  const handler = (event: any) => {
    console.log('[ProgressListen]', event.payload)

    const payload = event.payload

    if (isSelfcheck(payload)) {
      console.log('[ProgressListen] selfcheck received - ignored')
      return
    }

    const normalizedEvent = normalizeProgressEvent(payload)
    if (!normalizedEvent) {
      if (import.meta.env.DEV) {
        console.warn('[ProgressListen] drop event: invalid schema', payload)
      }
      return
    }

    store.applyEvent(normalizedEvent)

    if (!store.activeJobId) {
      const isJobInit = normalizedEvent.stepId === 'job.init'
      const isDownloadKickoff = normalizedEvent.stepId === 'driver.download' && normalizedEvent.state === 'running'
      if ((isJobInit || isDownloadKickoff) && normalizedEvent.jobId && normalizedEvent.jobId !== 'selfcheck') {
        store.setActiveJob(normalizedEvent.jobId)
        openInstallModal()
      }
    }
  }

  return {
    async start() {
      if (unlistenProgress) {
        console.log('[ProgressListen] 监听器已存在，跳过重复注册')
        return
      }
      try {
        unlistenProgress = await listen('install_progress', handler)
        console.log('[ProgressListen] 监听器注册成功')
      } catch (error) {
        console.error('[ProgressListen] 设置监听失败:', error)
      }
    },
    stop() {
      if (unlistenProgress) {
        unlistenProgress()
        unlistenProgress = null
        console.log('[ProgressListen] 监听器已注销')
      }
    },
  }
}
