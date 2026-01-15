/**
 * 安装进度相关的类型定义
 */

// 任务状态
export type JobState = 'queued' | 'running' | 'success' | 'failed' | 'canceled'

// 步骤状态
export type StepState = 'pending' | 'running' | 'success' | 'skipped' | 'failed'

// 后端事件类型（与后端 InstallProgressEvent 对应）
export interface InstallProgressEvent {
  jobId: string
  printerName: string
  stepId: string // 标准 stepId，例如 "driver.download"
  state: 'pending' | 'running' | 'success' | 'failed' | 'skipped'
  tsMs: number
  message?: string
  installMode?: string
  progress?: {
    current?: number
    total?: number
    unit?: 'bytes' | 'files' | 'percent'
    percent?: number
  }
  error?: {
    code: string
    detail: string
    stdout?: string
    stderr?: string
  }
  meta?: any
  legacyPhase?: string // 兼容旧前端
}

// 调试事件：用于记录被丢弃或非法的进度事件（不影响 UI）
export interface DebugEvent {
  at: number
  reason: string
  payload?: any
}

// 步骤快照
export interface StepSnapshot {
  stepId: string
  label: string
  state: StepState
  updatedAtMs?: number
  message?: string
  progress?: {
    current?: number
    total?: number
    percent?: number
    unit?: string
  }
  startedAt?: number
  endedAt?: number
  error?: {
    code: string
    detail: string
    stdout?: string
    stderr?: string
  }
  skipReason?: string
  meta?: any
}

// 安装任务快照
export interface InstallJobSnapshot {
  jobId: string
  printerName: string
  jobState: JobState
  startedAt?: number
  doneAtMs?: number
  updatedAt?: number
  steps: Record<string, StepSnapshot> // key 为 stepId
  debugEvents?: DebugEvent[]
  meta?: any // 任务元信息（如 installMode）
  // Watchdog 相关字段（用于处理缺终态事件）
  awaitingTerminal?: boolean
  terminalTimerId?: ReturnType<typeof setTimeout> | null
}
