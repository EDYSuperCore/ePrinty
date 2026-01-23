/**
 * 打印机运行时状态存储
 * 职责：
 * - 管理每台打印机的检测状态
 * - 管理安装方式选择
 * - 管理已安装的键值映射
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface PrinterRuntimeInfo {
  detectState: 'detecting' | 'installed' | 'not_installed' | 'unknown' | 'empty' | 'error' | 'cups_error'
  installedKey?: string | null
  systemQueueName?: string | null
  deviceUri?: string | null
  platform?: string | null
  displayName?: string | null
}

export const usePrinterRuntimeStore = defineStore('printerRuntime', () => {
  // 打印机运行时状态: key = printer.name
  const runtimeMap = ref<Record<string, PrinterRuntimeInfo>>({})

  // 打印机安装方式选择: key = printerKey (name__path)
  const installModeMap = ref<Record<string, string>>({})

  // 已安装的键值映射: key = printer.name, value = installedKey
  const installedKeyMap = ref<Record<string, string>>({})

  // 初始化
  function initializeRuntime(printers: any[]) {
    const map: Record<string, PrinterRuntimeInfo> = {}
    printers.forEach((printer) => {
      map[printer.name] = {
        detectState: 'detecting',
        installedKey: null,
        systemQueueName: null,
        deviceUri: null,
        platform: null,
        displayName: null,
      }
    })
    runtimeMap.value = map
  }

  // 更新检测状态
  function setDetectState(printerName: string, state: PrinterRuntimeInfo['detectState']) {
    if (runtimeMap.value[printerName]) {
      runtimeMap.value[printerName].detectState = state
    }
  }

  // 更新运行时信息
  function updateRuntime(printerName: string, info: Partial<PrinterRuntimeInfo>) {
    if (!runtimeMap.value[printerName]) {
      runtimeMap.value[printerName] = {
        detectState: 'unknown',
      }
    }
    Object.assign(runtimeMap.value[printerName], info)
  }

  // 设置安装方式
  function setInstallMode(printerKey: string, mode: string) {
    installModeMap.value[printerKey] = mode
  }

  // 获取安装方式
  function getInstallMode(printerKey: string): string | null {
    return installModeMap.value[printerKey] || null
  }

  // 设置已安装的键值
  function setInstalledKey(printerName: string, key: string) {
    installedKeyMap.value[printerName] = key
  }

  // 获取已安装的键值
  function getInstalledKey(printerName: string): string | null {
    return installedKeyMap.value[printerName] || null
  }

  // 加载已安装键值映射（从 localStorage）
  function loadInstalledKeyMap() {
    const stored = localStorage.getItem('eprinty_installed_key_map')
    if (stored) {
      try {
        installedKeyMap.value = JSON.parse(stored)
      } catch (err) {
        console.warn('Failed to parse installed key map:', err)
      }
    }
  }

  // 保存已安装键值映射（到 localStorage）
  function saveInstalledKeyMap() {
    localStorage.setItem('eprinty_installed_key_map', JSON.stringify(installedKeyMap.value))
  }

  // 重置
  function reset() {
    runtimeMap.value = {}
    installModeMap.value = {}
  }

  return {
    runtimeMap,
    installModeMap,
    installedKeyMap,
    initializeRuntime,
    setDetectState,
    updateRuntime,
    setInstallMode,
    getInstallMode,
    setInstalledKey,
    getInstalledKey,
    loadInstalledKeyMap,
    saveInstalledKeyMap,
    reset,
  }
})
