/**
 * 安装进度 Store（Pinia）
 * 以 jobId 作为唯一主键维护安装任务状态
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { InstallProgressEvent, InstallJobSnapshot, StepSnapshot } from '../models/installProgress'
import { createEmptyJob, calcJobPercent, getStepPlanForJob, normalizeInstallMode } from '../utils/progress'
import { ALLOWED_STATES, JOB_EVENT_IDS } from '../domain/progress/schema'

const allowedStateSet = new Set<string>(ALLOWED_STATES as readonly string[])
const jobEventSet = new Set<string>(JOB_EVENT_IDS as readonly string[])
const terminalStepStates = new Set<StepSnapshot['state']>(['success', 'failed', 'skipped'])
const terminalJobStates = new Set<InstallJobSnapshot['jobState']>(['success', 'failed', 'canceled'])

export const useInstallProgressStore = defineStore('installProgress', () => {
  // State
  const jobs = ref<Record<string, InstallJobSnapshot>>({})
  const activeJobId = ref<string | null>(null)
  // 用于在 jobId 返回前存储 meta（解决事件先到、invoke 后到的问题）
  const pendingMetaByPrinterName = ref<Record<string, any>>({})

  // Getters
  const activeJob = computed<InstallJobSnapshot | null>(() => {
    if (!activeJobId.value) {
      return null
    }
    return jobs.value[activeJobId.value] || null
  })

  const activeTotalPercent = computed(() => {
    const job = activeJob.value
    if (!job) {
      return 0
    }
    return calcJobPercent(job)
  })

  const activeStepsInOrder = computed<StepSnapshot[]>(() => {
    const job = activeJob.value
    if (!job) {
      return []
    }
    // 使用当前任务的步骤计划
    const stepPlan = getStepPlanForJob(job)
    return stepPlan.map(plan => job.steps[plan.stepId] || {
      stepId: plan.stepId,
      label: plan.label,
      state: 'pending' as const,
    })
  })

  const activeIsRunning = computed(() => {
    return activeJob.value?.jobState === 'running'
  })

  const activeIsFailed = computed(() => {
    return activeJob.value?.jobState === 'failed'
  })

  const activeIsSuccess = computed(() => {
    return activeJob.value?.jobState === 'success'
  })

  /**
   * 派生 UI 状态（统一真相源）
   * idle: 无活动任务
   * installing: 安装中
   * success: 安装成功
   * error: 安装失败
   */
  const deriveUIState = computed<'idle' | 'installing' | 'success' | 'error'>(() => {
    const job = activeJob.value
    if (!job) {
      return 'idle'
    }

    // 优先检查 job.done 步骤
    const doneStep = job.steps['job.done']
    if (doneStep) {
      if (doneStep.state === 'success') return 'success'
      if (doneStep.state === 'failed') return 'error'
    }

    // 根据 jobState 判断
    if (job.jobState === 'success') return 'success'
    if (job.jobState === 'failed' || job.jobState === 'canceled') return 'error'
    if (job.jobState === 'running') return 'installing'
    
    return 'idle'
  })

  /**
   * 派生进度百分比（与 overallProgressPercent 保持一致）
   */
  const derivePercent = computed(() => overallProgressPercent.value)

  /**
   * 活动任务的当前状态（或 'idle' 如果没有活动任务）
   */
  const activeJobState = computed(() => {
    return activeJob.value?.jobState || 'idle'
  })

  /**
   * 总体进度百分比（0-100）
   * 基于 displaySteps 计算，确保与 UI 显示的步骤列表一致
   */
  const overallProgressPercent = computed(() => {
    const steps = displaySteps.value
    
    if (!steps || steps.length === 0) {
      return 0
    }

    // 如果任务已成功，或 job.done 步骤为 success，则直接返回 100
    const doneStep = activeJob.value?.steps?.['job.done']
    if (activeIsSuccess.value || doneStep?.state === 'success') {
      return 100
    }
    
    let totalProgress = 0
    
    for (const step of steps) {
      let stepProgress = 0
      
      if (step.state === 'success' || step.state === 'skipped' || step.state === 'failed') {
        // 终态步骤记为 1 (100%)
        stepProgress = 1
      } else if (step.state === 'running') {
        // 运行中步骤：优先使用 progress.percent，否则默认 0.5
        if (step.progress?.percent !== undefined) {
          stepProgress = Math.min(step.progress.percent / 100, 1)
        } else if (step.progress?.total && step.progress.total > 0 && step.progress.current !== undefined) {
          stepProgress = Math.min(step.progress.current / step.progress.total, 1)
        } else {
          stepProgress = 0.5 // 运行中但无具体进度，默认 50%
        }
      } else {
        // pending 状态记为 0
        stepProgress = 0
      }
      
      totalProgress += stepProgress
    }
    
    // 计算百分比，保留 1 位小数
    const percent = Math.round((totalProgress / steps.length) * 1000) / 10
    
    // Clamp 到 0-100
    return Math.max(0, Math.min(100, percent))
  })

  /**
   * 仅展示步骤的有序数组（用于安装弹窗展示）
   * 基于当前任务的 installMode 动态决定步骤列表
   */
  const displaySteps = computed<StepSnapshot[]>(() => {
    const job = activeJob.value
    if (!job) {
      return []
    }
    
    // 使用当前任务的步骤计划
    const stepPlan = getStepPlanForJob(job)
    return stepPlan.map(plan => {
      const step = job.steps[plan.stepId]
      if (step) {
        return step
      }
      return {
        stepId: plan.stepId,
        label: plan.label,
        state: 'pending' as const,
      }
    })
  })

  /**
   * 活动任务的主要错误信息（如果失败）
   * 优先从失败的步骤中提取错误信息
   */
  const activePrimaryError = computed(() => {
    if (!activeIsFailed.value) {
      return ''
    }

    const job = activeJob.value
    if (!job) {
      return '安装失败：未知错误'
    }

    // 使用当前任务的步骤计划
    const stepPlan = getStepPlanForJob(job)
    for (const plan of stepPlan) {
      const step = job.steps[plan.stepId]
      if (step && step.state === 'failed') {
        if (step.error) {
          return step.error.detail || step.error.code || '未知错误'
        }
        if (step.message) {
          return step.message
        }
      }
    }

    return '安装失败：未知错误'
  })

  /**
   * 下载步骤的进度信息（driver.download）
   * 用于在 UI 中显示详细的下载进度
   */
  const activeDownloadStepProgress = computed(() => {
    const job = activeJob.value
    if (!job) return null
    
    const step = job.steps['driver.download']
    if (!step || step.state === 'pending') return null

    const current = step.progress?.current || 0
    const total = step.progress?.total || 0
    const percent = step.progress?.percent || 0

    return {
      state: step.state,
      message: step.message,
      currentBytes: current,
      totalBytes: total,
      percent: percent,
    }
  })

  // 判断事件内容是否与已有步骤完全一致（用于幂等去重）
  function isDuplicateStepEvent(step: StepSnapshot, evt: InstallProgressEvent): boolean {
    const sameState = step.state === evt.state
    const sameMsg = (step.message || '') === (evt.message || '')

    const existingProgress = step.progress || {}
    const incomingProgress = evt.progress || {}
    const sameProgress =
      (existingProgress.current ?? null) === (incomingProgress.current ?? null) &&
      (existingProgress.total ?? null) === (incomingProgress.total ?? null) &&
      (existingProgress.percent ?? null) === (incomingProgress.percent ?? null)

    const sameError =
      (step.error?.code || '') === (evt.error?.code || '') &&
      (step.error?.detail || '') === (evt.error?.detail || '')

    return sameState && sameMsg && sameProgress && sameError
  }

  // Actions
  /**
   * 设置待处理的任务 meta（在 jobId 返回前调用）
   */
  function setPendingJobMeta(printerName: string, meta: any) {
    pendingMetaByPrinterName.value[printerName] = meta
    console.log(`[InstallProgressStore] setPendingJobMeta: printerName=${printerName}`, meta)
  }

  /**
   * 确保任务存在（如果不存在则创建）
   */
  function ensureJob(jobId: string, printerName: string): InstallJobSnapshot {
    if (!jobs.value[jobId]) {
      // 合并 pending meta（如果有）
      const pendingMeta = pendingMetaByPrinterName.value[printerName]
      const meta = pendingMeta ? { ...pendingMeta } : {}
      
      jobs.value[jobId] = createEmptyJob(jobId, printerName, meta)
      console.log(`[InstallProgressStore] 创建新任务 jobId=${jobId} printerName=${printerName}`, { meta })
      
      // 清理 pending meta
      if (pendingMeta) {
        delete pendingMetaByPrinterName.value[printerName]
      }
    }
    return jobs.value[jobId]
  }

  /**
   * 应用进度事件（Reducer）
   */
  function applyEvent(evt: InstallProgressEvent) {
    // 必须使用 evt.jobId 作为 key
    if (!evt.jobId || evt.jobId.trim() === '') {
      console.warn('[InstallProgressStore] 丢弃事件：缺少 jobId', evt)
      return
    }

    // 确保任务存在
    const job = ensureJob(evt.jobId, evt.printerName)

    const incomingTs = typeof evt.tsMs === 'number' ? evt.tsMs : Date.now()

    const stepId = (evt.stepId || '').trim()
    if (!stepId) {
      pushDebugEvent(job, 'missing stepId', evt)
      return
    }

    if (!allowedStateSet.has(evt.state)) {
      pushDebugEvent(job, `invalid state ${evt.state}`, evt)
      return
    }

    // job 终态保护：一旦终态，忽略后续 pending/running 事件对 jobState 的影响
    if (job.doneAtMs != null && terminalJobStates.has(job.jobState) && (evt.state === 'running' || evt.state === 'pending')) {
      pushDebugEvent(job, 'job already in terminal state, ignore non-terminal event', evt)
      return
    }

    // 判断是 job 级别事件还是展示步骤
    const isJobEvent = jobEventSet.has(stepId)
    
    // 检查是否属于当前任务的步骤计划
    const jobStepPlan = getStepPlanForJob(job)
    const isDisplayStep = jobStepPlan.some(plan => plan.stepId === stepId)

    if (!isJobEvent && !isDisplayStep) {
      // 未知步骤：记录但不展示，并做幂等去重
      const existing = job.steps[stepId]
      if (existing && isDuplicateStepEvent(existing, evt)) {
        return
      }

      const target = existing || {
        stepId,
        label: stepId,
        state: 'pending' as const,
        updatedAtMs: incomingTs,
      }

      target.state = evt.state as StepSnapshot['state']
      target.updatedAtMs = incomingTs
      if (['success', 'skipped', 'failed'].includes(evt.state)) {
        target.endedAt = incomingTs
      }
      if (evt.message) {
        target.message = evt.message
      }
      if (evt.progress) {
        target.progress = {
          current: evt.progress.current,
          total: evt.progress.total,
          percent: evt.progress.percent,
          unit: evt.progress.unit,
        }
      }
      if (evt.state === 'failed' && evt.error) {
        target.error = {
          code: evt.error.code,
          detail: evt.error.detail,
          stdout: evt.error.stdout,
          stderr: evt.error.stderr,
        }
      }

      if (!existing) {
        job.steps[stepId] = target
      }

      pushDebugEvent(job, `unknown stepId ${stepId}`, evt)
      job.updatedAt = incomingTs
      return
    }

    if (isJobEvent) {
      // Job 级别事件：只更新 job 元信息，不进入步骤列表
      if (stepId === 'job.init' && evt.state === 'running') {
        job.startedAt = incomingTs
        job.jobState = 'running'
        
        // 从 job.init 事件提取 installMode/driverKey，写入 job.meta（关键！）
        const rawInstallMode = evt.installMode || evt.meta?.installMode
        if (rawInstallMode) {
          if (!job.meta) {
            job.meta = {}
          }
          const normalized = normalizeInstallMode(rawInstallMode as string)
          job.meta.installMode = normalized
          console.log(`[InstallProgressStore] job.init: extracted installMode="${rawInstallMode}" normalized="${normalized}"`)
        }

        if (evt.meta) {
          if (!job.meta) {
            job.meta = {}
          }
          // 提取 driverKey
          const driverKey = evt.meta.driverKey
          if (driverKey) {
            job.meta.driverKey = driverKey
            console.log(`[InstallProgressStore] job.init: extracted driverKey="${driverKey}"`)
          }

          const queueName = evt.meta.queueName
          if (queueName) {
            job.meta.queueName = queueName
          }

          const deviceUri = evt.meta.deviceUri || evt.meta.uri
          if (deviceUri) {
            job.meta.deviceUri = deviceUri
          }
        }
      } else if (stepId === 'job.done' || stepId === 'job.failed') {
        // 清理 watchdog timer（如果还在等待终态）
        if (job.terminalTimerId) {
          clearTimeout(job.terminalTimerId)
          job.terminalTimerId = null
        }
        job.awaitingTerminal = false

        job.doneAtMs = incomingTs
        job.jobState = stepId === 'job.failed' ? 'failed' : evt.state === 'success' ? 'success' : 'failed'

        // 写入终态步骤快照（不展示，用于终态判断）
        const doneStepId = stepId === 'job.failed' ? 'job.failed' : 'job.done'
        const existingDone = job.steps[doneStepId]
        if (!existingDone || !isDuplicateStepEvent(existingDone, evt)) {
          job.steps[doneStepId] = {
            stepId: doneStepId,
            label: doneStepId === 'job.failed' ? '安装失败' : '安装完成',
            state: stepId === 'job.failed' ? 'failed' : (evt.state as StepSnapshot['state']),
            message: evt.message,
            endedAt: incomingTs,
            updatedAtMs: incomingTs,
          }
        }

        job.updatedAt = incomingTs
      }
      // job 级别事件不进入步骤列表，直接返回
      return
    }

    // Step 级别事件：更新步骤列表
    // 获取或创建步骤快照
    let step = job.steps[stepId]
    if (!step) {
      // 从当前任务的步骤计划中查找标签
      const jobStepPlan = getStepPlanForJob(job)
      const planItem = jobStepPlan.find(p => p.stepId === stepId)
      step = {
        stepId,
        label: planItem?.label || stepId,
        state: 'pending',
        updatedAtMs: incomingTs,
      }
      job.steps[stepId] = step
    }

    // 幂等去重：内容完全一致则忽略
    if (step && isDuplicateStepEvent(step, evt)) {
      return
    }

    // 终态保护：终态后收到 pending/running 事件不回滚状态
    const isStepTerminal = terminalStepStates.has(step.state)
    const isIncomingNonTerminal = evt.state === 'running' || evt.state === 'pending'
    if (isStepTerminal && isIncomingNonTerminal) {
      pushDebugEvent(job, `ignore non-terminal event on terminal step ${stepId}`, evt)
      return
    }

    // 时间戳保护：旧事件不覆盖新状态
    if (typeof step.updatedAtMs === 'number' && incomingTs < step.updatedAtMs) {
      pushDebugEvent(job, `stale event tsMs=${incomingTs} < updatedAtMs=${step.updatedAtMs} for step ${stepId}`, evt)
      return
    }

    // 更新步骤状态
    const oldState = step.state
    step.state = evt.state as StepSnapshot['state']
    step.updatedAtMs = incomingTs

    // 更新 startedAt（第一次进入 running 时）
    if (oldState === 'pending' && evt.state === 'running') {
      step.startedAt = incomingTs
    }

    // 更新 endedAt（进入终态时）
    if (['success', 'skipped', 'failed'].includes(evt.state)) {
      step.endedAt = incomingTs
    }

    // 更新 message
    if (evt.message) {
      step.message = evt.message
    }

    // 更新 progress
    if (evt.progress) {
      step.progress = {
        current: evt.progress.current,
        total: evt.progress.total,
        percent: evt.progress.percent,
        unit: evt.progress.unit,
      }
    }

    // 更新 error（仅在 failed 时）
    if (evt.state === 'failed' && evt.error) {
      step.error = {
        code: evt.error.code,
        detail: evt.error.detail,
        stdout: evt.error.stdout,
        stderr: evt.error.stderr,
      }
    }

    // 更新 skipReason（仅在 skipped 时）
    if (evt.state === 'skipped' && evt.message) {
      step.skipReason = evt.message
    }

    // 更新 meta
    if (evt.meta) {
      step.meta = evt.meta
    }

    // 更新任务状态
    updateJobState(job)

    // 更新 updatedAt
    job.updatedAt = incomingTs
  }

  /**
   * 更新任务状态（基于所有步骤的状态）
   */
  function updateJobState(job: InstallJobSnapshot) {
    // 终态以 job.done 为准
    const doneStep = job.steps['job.done']
    if (doneStep && (doneStep.state === 'success' || doneStep.state === 'failed')) {
      job.jobState = doneStep.state === 'success' ? 'success' : 'failed'
      return
    }

    // 使用当前任务的步骤计划
    const stepPlan = getStepPlanForJob(job)

    // 检查是否有失败的步骤
    const hasFailed = stepPlan.some(plan => {
      const step = job.steps[plan.stepId]
      return step?.state === 'failed'
    })

    // 检查是否有运行中的步骤
    const hasRunning = stepPlan.some(plan => {
      const step = job.steps[plan.stepId]
      return step?.state === 'running'
    })

    // 检查是否所有步骤都完成（success 或 skipped）
    const allCompleted = stepPlan.every(plan => {
      const step = job.steps[plan.stepId]
      return step && (step.state === 'success' || step.state === 'skipped')
    })

    // 更新 jobState
    if (hasFailed) {
      job.jobState = 'failed'
    } else if (allCompleted) {
      job.jobState = 'success'
    } else if (hasRunning || job.jobState === 'running') {
      job.jobState = 'running'
    } else if (job.jobState === 'queued') {
      // 保持 queued 状态，直到有步骤开始运行
    }
  }

  /**
   * 取消安装任务
   */
  function cancelJob(jobId: string) {
    const job = jobs.value[jobId]
    if (!job) {
      console.warn(`[InstallProgressStore] cancelJob: job not found jobId=${jobId}`)
      return
    }

    // 标记为已取消
    job.jobState = 'canceled'
    job.updatedAt = Date.now()

    // 清理 watchdog
    if (job.terminalTimerId) {
      clearTimeout(job.terminalTimerId)
      job.terminalTimerId = null
    }
    job.awaitingTerminal = false

    // 将当前 running 步骤标记为 failed
    const stepPlan = getStepPlanForJob(job)
    for (const plan of stepPlan) {
      const step = job.steps[plan.stepId]
      if (step && step.state === 'running') {
        step.state = 'failed'
        step.message = '用户取消安装'
        step.endedAt = Date.now()
        step.updatedAtMs = Date.now()
      }
    }

    console.log(`[InstallProgressStore] cancelJob: job=${jobId} canceled by user`)
  }

  /**
   * 设置当前活动的任务 ID
   */
  function setActiveJob(jobId: string | null) {
    activeJobId.value = jobId
    if (jobId) {
      console.log(`[InstallProgressStore] 设置活动任务 jobId=${jobId}`)
    }
  }

  /**
   * 清除任务（可选，用于清理）
   */
  function clearJob(jobId: string) {
    delete jobs.value[jobId]
    if (activeJobId.value === jobId) {
      activeJobId.value = null
    }
  }

  /**
   * 从 invoke 返回结果兜底 finalize（确保 job 收敛到终态）
   * 即使后端漏发终态事件，也能通过 invoke 结果收敛
   */
  function finalizeFromInvoke(
    jobId: string,
    invokeResult: { success: boolean; message?: string; job_id?: string }
  ) {
    if (!jobId && invokeResult.job_id) {
      jobId = invokeResult.job_id
    }
    if (!jobId) {
      console.warn('[InstallProgressStore] finalizeFromInvoke: missing jobId')
      return
    }

    const job = jobs.value[jobId]
    if (!job) {
      console.warn(`[InstallProgressStore] finalizeFromInvoke: job not found jobId=${jobId}`)
      return
    }

    const now = Date.now()

    if (invokeResult.success && jobId && jobId !== '(未返回)') {
      // 记录 invoke 成功的旁证信息（不改写 steps / jobState）
      job.meta = {
        ...job.meta,
        invokeSuccess: true,
        invokeMessage: invokeResult.message,
        invokeAt: now,
      }

      // Check if job already reached terminal state (job.done already arrived)
      // Check both job-level flags and job.done step status
      const doneStep = job.steps?.['job.done']
      const isDoneStepTerminal = doneStep && (doneStep.state === 'success' || doneStep.state === 'failed')
      
      if (job.doneAtMs || job.jobState === 'success' || job.jobState === 'failed' || isDoneStepTerminal) {
        console.log(`[InstallProgressStore] finalizeFromInvoke: job=${jobId} already terminal; skip watchdog`, {
          jobState: job.jobState,
          doneAtMs: job.doneAtMs,
          doneStepState: doneStep?.state
        })
        return
      }

      // Invoke succeeded and we have a valid jobId: set up watchdog timer.
      // The job state is controlled by terminal events (job.init, job.done).
      // We defer the failure decision to allow step inference and job.done to arrive.
      // If terminal events don't arrive within TERMINAL_EVENT_TIMEOUT_MS, the watchdog will mark job as failed.

      // Clear any existing watchdog timer
      if (job.terminalTimerId) {
        clearTimeout(job.terminalTimerId)
      }

      // Mark job as awaiting terminal events
      job.awaitingTerminal = true

      // Set up watchdog timer: if job.done doesn't arrive in 12 seconds, mark job as failed
      const timerId = setTimeout(() => {
        const j = jobs.value[jobId]
        
        // Defense: check if job still needs watchdog action
        if (!j || !j.awaitingTerminal) {
          return
        }
        
        // Defense: check if job already reached terminal state
        // Also check job.done step status in case it arrived but flags not updated yet
        const doneStep = j.steps?.['job.done']
        const isDoneStepTerminal = doneStep && (doneStep.state === 'success' || doneStep.state === 'failed')
        
        if (j.doneAtMs || j.jobState === 'success' || j.jobState === 'failed' || isDoneStepTerminal) {
          console.debug(`[InstallProgressStore] watchdog: job=${jobId} already terminal at timeout; skip action`, {
            doneStepState: doneStep?.state
          })
          return
        }
        
        console.warn(`[InstallProgressStore] watchdog timeout: job=${jobId} did not receive terminal events within 12s`)
        j.jobState = 'failed'

        // 使用当前任务的步骤计划查找运行中的步骤
        const stepPlan = getStepPlanForJob(j)
        let runningStep: StepSnapshot | null = null
        for (const plan of stepPlan) {
          const step = j.steps[plan.stepId]
          if (step && step.state === 'running') {
            runningStep = step
            break
          }
        }

        if (runningStep) {
          runningStep.state = 'failed'
          runningStep.message = '终态事件超时'
          runningStep.error = {
            code: 'TERMINAL_EVENT_TIMEOUT',
            detail: '等待终态事件超时，无法确认完成',
            stdout: undefined,
            stderr: undefined,
          }
          runningStep.endedAt = now
          runningStep.updatedAtMs = now
        }

        j.awaitingTerminal = false
        j.terminalTimerId = undefined
      }, 12000) // 12 second timeout

      job.terminalTimerId = timerId
      pushDebugEvent(job, 'invoke success; watching for terminal events', {
        success: invokeResult.success,
        message: invokeResult.message,
        jobId: jobId,
        awaitingTerminal: true
      })
      console.log(`[InstallProgressStore] finalizeFromInvoke: job=${jobId} invoke succeeded; waiting for terminal events (12s timeout)`)
    } else if (invokeResult.success && (!jobId || jobId === '(未返回)')) {
      // Invoke succeeded but jobId missing: log and do not set failed status
      // (store will infer success from terminal events or fail via watchdog if they don't arrive)
      pushDebugEvent(job, 'invoke success but jobId missing; terminal events cannot be correlated', {
        success: invokeResult.success,
        message: invokeResult.message,
        jobId: jobId
      })
      console.warn('[InstallProgressStore] finalizeFromInvoke: invoke success but jobId missing; cannot set up watchdog')
    } else {
      // 失败：强制设为 failed，标记当前 running step 为 failed
      job.jobState = 'failed'
      job.updatedAt = now

      // 使用当前任务的步骤计划查找运行中的步骤
      const stepPlan = getStepPlanForJob(job)
      let lastRunningStep: StepSnapshot | null = null
      for (const plan of stepPlan) {
        const step = job.steps[plan.stepId]
        if (step && step.state === 'running') {
          lastRunningStep = step
        }
      }

      if (lastRunningStep) {
        lastRunningStep.state = 'failed'
        lastRunningStep.message = invokeResult.message || '安装失败'
        lastRunningStep.error = {
          code: 'INVOKE_FAILED',
          detail: invokeResult.message || '安装失败',
          stdout: undefined,
          stderr: undefined,
        }
        lastRunningStep.endedAt = now
        lastRunningStep.updatedAtMs = now
      } else {
        // 如果没有 running step，标记第一个 pending step 为 failed
        for (const plan of stepPlan) {
          const step = job.steps[plan.stepId]
          if (step && step.state === 'pending') {
            step.state = 'failed'
            step.message = invokeResult.message || '安装失败'
            step.error = {
              code: 'INVOKE_FAILED',
              detail: invokeResult.message || '安装失败',
              stdout: undefined,
              stderr: undefined,
            }
            step.endedAt = now
            step.updatedAtMs = now
            break
          }
        }
      }

      console.log(`[InstallProgressStore] finalizeFromInvoke: job=${jobId} finalized to failed`)
    }
  }

  /**
   * 记录调试事件（非法/丢弃的进度事件）
   */
  function pushDebugEvent(job: InstallJobSnapshot, reason: string, payload: any) {
    if (!job.debugEvents) {
      job.debugEvents = []
    }
    job.debugEvents.push({
      at: Date.now(),
      reason,
      payload,
    })
    if (job.debugEvents.length > 200) {
      job.debugEvents.shift()
    }
  }

  return {
    // State
    jobs,
    activeJobId,
    // Getters
    activeJob,
    activeTotalPercent,
    activeStepsInOrder,
    activeIsRunning,
    activeIsFailed,
    activeIsSuccess,
    activeJobState,
    overallProgressPercent,
    displaySteps,
    activePrimaryError,
    activeDownloadStepProgress,
    deriveUIState,
    derivePercent,
    // Actions
    setPendingJobMeta,
    ensureJob,
    applyEvent,
    setActiveJob,
    clearJob,
    cancelJob,
    finalizeFromInvoke,
    pushDebugEvent,
  }
})
