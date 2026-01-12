/**
 * 安装进度相关的工具函数
 */

import type { StepSnapshot, InstallJobSnapshot } from '../models/installProgress'

// 安装模式类型
export type InstallMode = 'auto' | 'package_only' | 'pnputil_only' | 'powershell_only' | 'inf_only'

// 步骤计划类型
export interface StepPlan {
  stepId: string
  label: string
  weight: number
}

// 按模式分组的步骤计划
export const STEP_PLANS_BY_MODE: Record<InstallMode, StepPlan[]> = {
  // auto 模式：包含完整下载流程
  auto: [
    { stepId: 'driver.download', label: '下载驱动包', weight: 15 },
    { stepId: 'driver.verify', label: '校验驱动包', weight: 5 },
    { stepId: 'driver.extract', label: '解压/合并文件', weight: 10 },
    { stepId: 'driver.stageDriver', label: '注册到 DriverStore', weight: 20 },
    { stepId: 'driver.registerDriver', label: '注册打印驱动', weight: 20 },
    { stepId: 'device.ensurePort', label: '创建/校验端口', weight: 10 },
    { stepId: 'device.ensureQueue', label: '创建/校验队列', weight: 10 },
    { stepId: 'device.finalVerify', label: '最终验证', weight: 10 },
  ],
  // package_only 模式：不含下载流程
  package_only: [
    { stepId: 'driver.stageDriver', label: '注册到 DriverStore', weight: 20 },
    { stepId: 'driver.registerDriver', label: '注册打印驱动', weight: 20 },
    { stepId: 'device.ensurePort', label: '创建/校验端口', weight: 20 },
    { stepId: 'device.ensureQueue', label: '创建/校验队列', weight: 20 },
    { stepId: 'device.finalVerify', label: '最终验证', weight: 20 },
  ],
  // pnputil_only 模式：使用与 package_only 相同的流程
  pnputil_only: [
    { stepId: 'driver.stageDriver', label: '注册到 DriverStore', weight: 20 },
    { stepId: 'driver.registerDriver', label: '注册打印驱动', weight: 20 },
    { stepId: 'device.ensurePort', label: '创建/校验端口', weight: 20 },
    { stepId: 'device.ensureQueue', label: '创建/校验队列', weight: 20 },
    { stepId: 'device.finalVerify', label: '最终验证', weight: 20 },
  ],
  // powershell_only 模式：使用与 package_only 相同的流程
  powershell_only: [
    { stepId: 'driver.stageDriver', label: '注册到 DriverStore', weight: 20 },
    { stepId: 'driver.registerDriver', label: '注册打印驱动', weight: 20 },
    { stepId: 'device.ensurePort', label: '创建/校验端口', weight: 20 },
    { stepId: 'device.ensureQueue', label: '创建/校验队列', weight: 20 },
    { stepId: 'device.finalVerify', label: '最终验证', weight: 20 },
  ],
  // inf_only 模式：使用与 package_only 相同的流程
  inf_only: [
    { stepId: 'driver.stageDriver', label: '注册到 DriverStore', weight: 20 },
    { stepId: 'driver.registerDriver', label: '注册打印驱动', weight: 20 },
    { stepId: 'device.ensurePort', label: '创建/校验端口', weight: 20 },
    { stepId: 'device.ensureQueue', label: '创建/校验队列', weight: 20 },
    { stepId: 'device.finalVerify', label: '最终验证', weight: 20 },
  ],
}

// 默认步骤计划（用于向后兼容）
export const STEPS_PLAN = STEP_PLANS_BY_MODE.package_only

/**
 * 规范化安装模式（兼容历史值和后端值）
 * 后端返回 "package"，前端统一为 "package_only"
 * 后端返回 "auto"，前端统一为 "auto"
 */
export function normalizeInstallMode(mode?: string | null): InstallMode {
  if (!mode || typeof mode !== 'string') {
    return 'auto'
  }

  const trimmed = mode.trim().toLowerCase()

  // 兼容历史值和后端值 'package' -> 'package_only'
  if (trimmed === 'package') {
    return 'package_only'
  }

  // 校验是否为合法模式
  if (trimmed in STEP_PLANS_BY_MODE) {
    return trimmed as InstallMode
  }

  // 不识别的模式默认为 auto
  console.warn(`[normalizeInstallMode] Unknown mode "${mode}", fallback to auto`)
  return 'auto'
}

/**
 * 获取任务的步骤计划
 */
export function getStepPlanForJob(job: InstallJobSnapshot): StepPlan[] {
  const mode = normalizeInstallMode(job.meta?.installMode)
  return STEP_PLANS_BY_MODE[mode]
}

/**
 * 规范化 stepId（兼容历史 phase 名称）
 */
export function normalizeStepId(input: string | null | undefined): string {
  if (!input || typeof input !== 'string') {
    return ''
  }

  const trimmed = input.trim()

  // 如果已经是标准格式（driver.xxx 或 device.xxx），直接返回
  if (trimmed.startsWith('driver.') || trimmed.startsWith('device.')) {
    return trimmed
  }

  // 映射旧 phase 名称到新 stepId
  const phaseMap: Record<string, string> = {
    'download': 'driver.download',
    'verify': 'driver.verify',
    'extract': 'driver.extract',
    'stageDriver': 'driver.stageDriver',
    'registerDriver': 'driver.registerDriver',
    'ensurePort': 'device.ensurePort',
    'ensureQueue': 'device.ensureQueue',
    'finalVerify': 'device.finalVerify',
  }

  return phaseMap[trimmed] || trimmed
}

/**
 * 创建空的安装任务快照
 */
export function createEmptyJob(
  jobId: string,
  printerName: string,
  meta?: any
): InstallJobSnapshot {
  const job: InstallJobSnapshot = {
    jobId,
    printerName,
    jobState: 'queued',
    steps: {},
    startedAt: Date.now(),
    updatedAt: Date.now(),
    debugEvents: [],
    meta: meta || {},
  }

  // 根据模式初始化对应的步骤为 pending
  const stepPlan = getStepPlanForJob(job)
  for (const plan of stepPlan) {
    job.steps[plan.stepId] = {
      stepId: plan.stepId,
      label: plan.label,
      state: 'pending',
    }
  }

  return job
}

/**
 * 计算任务总进度（权重聚合）
 */
export function calcJobPercent(job: InstallJobSnapshot): number {
  let totalWeight = 0
  let weightedProgress = 0

  // 使用当前任务的步骤计划
  const stepPlan = getStepPlanForJob(job)

  for (const plan of stepPlan) {
    const step = job.steps[plan.stepId]
    if (!step) {
      continue
    }

    let p = 0

    if (step.state === 'success' || step.state === 'skipped' || step.state === 'failed') {
      p = 1
    } else if (step.state === 'running') {
      if (step.progress?.total && step.progress.total > 0 && step.progress.current !== undefined) {
        p = Math.min(step.progress.current / step.progress.total, 1)
      } else if (step.progress?.percent !== undefined) {
        p = Math.min(step.progress.percent / 100, 1)
      } else {
        p = 0.5
      }
    } else {
      p = 0
    }

    totalWeight += plan.weight
    weightedProgress += plan.weight * p
  }

  if (totalWeight === 0) {
    return 0
  }

  // 返回 0-100 的数值，保留 1 位小数
  return Math.round((weightedProgress / totalWeight) * 1000) / 10
}

/**
 * 格式化字节数
 */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes === 0) {
    return '0 B'
  }

  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}
