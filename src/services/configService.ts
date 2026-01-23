/**
 * 配置管理服务
 * 职责：
 * - loadCachedConfig: 读取缓存配置（本地优先）
 * - refreshRemoteConfig: 后台刷新远程配置
 * - 事件发送：config_updated, config_refresh_failed
 */

import { invoke } from '@tauri-apps/api/tauri'

export interface PrinterConfig {
  version?: string
  cities: Array<any>
  [key: string]: any
}

export interface CachedConfigResult {
  config: PrinterConfig
  source: string // 'local' | 'cache' | 'seed' | 'remote_bootstrap'
  timestamp?: number
  version?: string
}

export interface RefreshConfigResult {
  success: boolean
  error?: string
  version?: string
}

/**
 * 读取缓存配置（本地优先）
 */
export async function loadCachedConfig(): Promise<CachedConfigResult> {
  const result = await invoke<CachedConfigResult>('get_cached_config')
  return result
}

/**
 * 刷新远程配置（后台异步，不阻塞）
 */
export async function refreshRemoteConfig(): Promise<RefreshConfigResult> {
  const result = await invoke<RefreshConfigResult>('refresh_remote_config')
  return result
}

/**
 * 配置服务初始化状态
 */
export interface ConfigLoadState {
  initialSource: string | null
  refreshing: boolean
  lastRefreshError: string | null
}

export function createConfigLoadState(): ConfigLoadState {
  return {
    initialSource: null,
    refreshing: false,
    lastRefreshError: null,
  }
}
