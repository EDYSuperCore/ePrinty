/**
 * 打印机检测服务
 * 职责：
 * - startDetectInstalledPrinters：异步启动检测
 * - 自动重试机制
 * - 超时处理
 */

import { invoke } from '@tauri-apps/api/tauri'

export interface DetectState {
  status: 'idle' | 'running' | 'timeout' | 'error'
  error: string | null
}

export function createDetectState(): DetectState {
  return {
    status: 'idle',
    error: null,
  }
}

export interface PrinterDetectEntry {
  installedKey: string
  systemQueueName: string
  displayName?: string
  deviceUri?: string
  platform?: string
}

/**
 * 启动打印机检测（带重试机制）
 */
export async function startDetectInstalledPrinters(
  timeoutMs: number = 8000,
  maxAttempts: number = 2
): Promise<PrinterDetectEntry[]> {
  const detectId = `detect_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  const detectStartTime = performance.now()

  let attemptCount = 0

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    attemptCount = attempt
    const attemptStartTime = performance.now()
    const currentTimeout = attempt === 1 ? timeoutMs : timeoutMs * 2

    console.log(
      `[PrinterDetect] ATTEMPT_START detect_id=${detectId} attempt=${attempt} timeout_ms=${currentTimeout}`
    )

    let timeoutId: NodeJS.Timeout | null = null
    let detectCompleted = false

    try {
      const detectPromise = invoke<PrinterDetectEntry[]>('list_printers')

      const timeoutPromise = new Promise<null>((resolve) => {
        timeoutId = setTimeout(() => {
          if (!detectCompleted) {
            console.log(
              `[PrinterDetect] TIMEOUT_TRIGGERED detect_id=${detectId} attempt=${attempt} timeout_ms=${currentTimeout}`
            )
            resolve(null)
          }
        }, currentTimeout)
      })

      const result = await Promise.race([detectPromise, timeoutPromise])

      detectCompleted = true
      if (timeoutId !== null) {
        clearTimeout(timeoutId)
        timeoutId = null
      }

      const attemptElapsed = performance.now() - attemptStartTime

      if (result === null) {
        console.log(
          `[PrinterDetect] ATTEMPT_TIMEOUT detect_id=${detectId} attempt=${attempt} elapsed_ms=${attemptElapsed.toFixed(
            2
          )}`
        )

        if (attempt < maxAttempts) {
          console.log(
            `[PrinterDetect] AUTO_RETRY detect_id=${detectId} next_attempt=${attempt + 1}`
          )
          continue
        } else {
          throw new Error('Detection timeout')
        }
      } else if (Array.isArray(result)) {
        console.log(
          `[PrinterDetect] INVOKE_RESOLVE detect_id=${detectId} attempt=${attempt} result_length=${result.length} elapsed_ms=${attemptElapsed.toFixed(
            2
          )}`
        )
        return result
      } else {
        throw new Error('Invalid result format')
      }
    } catch (err) {
      detectCompleted = true
      if (timeoutId !== null) {
        clearTimeout(timeoutId)
        timeoutId = null
      }

      const attemptElapsed = performance.now() - attemptStartTime
      console.log(
        `[PrinterDetect] ATTEMPT_ERROR detect_id=${detectId} attempt=${attempt} elapsed_ms=${attemptElapsed.toFixed(
          2
        )} error=${err}`
      )

      if (attempt < maxAttempts) {
        console.log(`[PrinterDetect] AUTO_RETRY detect_id=${detectId} next_attempt=${attempt + 1}`)
        continue
      } else {
        const totalElapsed = performance.now() - detectStartTime
        console.log(
          `[PrinterDetect] DETECT_FINAL_ERROR detect_id=${detectId} total_elapsed_ms=${totalElapsed.toFixed(
            2
          )} attempts=${attemptCount}`
        )
        throw err
      }
    }
  }

  throw new Error('Detection failed after all attempts')
}
