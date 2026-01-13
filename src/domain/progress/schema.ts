import { normalizeStepId } from '../../utils/progress'
import type { InstallProgressEvent as EventType } from '../../models/installProgress'

// 可展示步骤 ID（需与后端保持一致）
export const DISPLAY_STEP_IDS = [
  'device.probe',
  'driver.download',
  'driver.verify',
  'driver.extract',
  'driver.stageDriver',
  'driver.registerDriver',
  'device.ensurePort',
  'device.ensureQueue',
  'device.finalVerify',
] as const

// Job 级别事件 ID（不进入步骤列表）
export const JOB_EVENT_IDS = ['job.init', 'job.done'] as const

export const ALLOWED_STEP_IDS = [...DISPLAY_STEP_IDS, ...JOB_EVENT_IDS] as const

export const ALLOWED_STATES = ['pending', 'running', 'success', 'failed', 'skipped'] as const

const allowedStateSet = new Set<string>(ALLOWED_STATES as readonly string[])

export type NormalizedProgressEvent = EventType

export function normalizeProgressEvent(payload: any): NormalizedProgressEvent | null {
  const rawJobId = typeof payload?.jobId === 'string' ? payload.jobId : typeof payload?.job_id === 'string' ? payload.job_id : ''
  const rawStepId = typeof payload?.stepId === 'string'
    ? payload.stepId
    : typeof payload?.step_id === 'string'
      ? payload.step_id
      : typeof payload?.legacyPhase === 'string'
        ? payload.legacyPhase
        : typeof payload?.legacy_phase === 'string'
          ? payload.legacy_phase
          : ''

  if (!rawJobId.trim()) {
    return null
  }
  if (!rawStepId.trim()) {
    return null
  }

  const normalizedStepId = normalizeStepId(rawStepId)
  if (!normalizedStepId) {
    return null
  }

  const state = payload?.state || 'running'
  if (!allowedStateSet.has(state)) {
    return null
  }

  const tsMs = typeof payload?.tsMs === 'number'
    ? payload.tsMs
    : typeof payload?.ts_ms === 'number'
      ? payload.ts_ms
      : Date.now()

  const normalized: NormalizedProgressEvent = {
    jobId: rawJobId,
    printerName: payload?.printerName || payload?.printer_name || '',
    stepId: normalizedStepId,
    state,
    tsMs,
    message: payload?.message || undefined,
    progress: payload?.progress
      ? {
          current: payload.progress.current,
          total: payload.progress.total,
          unit: payload.progress.unit,
          percent: payload.progress.percent,
        }
      : undefined,
    error: payload?.error
      ? {
          code: payload.error.code || '',
          detail: payload.error.detail || '',
          stdout: payload.error.stdout,
          stderr: payload.error.stderr,
        }
      : undefined,
    meta: payload?.meta,
    legacyPhase: payload?.legacyPhase || payload?.legacy_phase,
  }

  return normalized
}
