/**
 * 打印机安装服务
 * 统一管理安装请求的参数校验、调用和结果处理
 */

import { invoke } from '@tauri-apps/api/tauri'

/**
 * 安装请求参数
 * v2.0.0+：使用 driverKey 替代 driverPath，后端从 driverCatalog 自动获取驱动规格
 */
export interface InstallRequest {
  name: string
  path: string
  driverKey?: string  // v2.0.0+：驱动键（必填）
  driverPath?: string | null  // 向后兼容（不推荐）
  model?: string | null
  driverInstallPolicy?: string
  installMode: string
  dryRun?: boolean
}

/**
 * 安装响应结果
 */
export interface InstallResponse {
  success: boolean
  jobId?: string
  message: string
  method?: string
  effectiveDryRun?: boolean
  stdout?: string
  stderr?: string
  details?: any
}

/**
 * 参数校验错误
 */
class InstallValidationError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'InstallValidationError'
  }
}

/**
 * 校验安装模式是否合法
 * 必须与后端 install_mode 支持的值一致：main.rs Printer.install_mode
 */
function validateInstallMode(mode: string): void {
  const validModes = [
    'auto',
    'package',
    'installer',
    'ipp',
    'legacy_inf'
  ]
  
  if (!mode) {
    throw new InstallValidationError('installMode 是必填项')
  }
  
  if (!validModes.includes(mode)) {
    throw new InstallValidationError(
      `installMode "${mode}" 不合法。有效值: ${validModes.join(', ')}`
    )
  }
}

/**
 * 校验安装请求参数
 */
function validateInstallRequest(req: InstallRequest): void {
  // 必填字段
  if (!req.name || req.name.trim() === '') {
    throw new InstallValidationError('打印机名称 (name) 不能为空')
  }
  
  if (!req.path || req.path.trim() === '') {
    throw new InstallValidationError('打印机路径 (path) 不能为空')
  }
  
  // 校验 installMode
  validateInstallMode(req.installMode)
  
  // 依赖关系检查（如果有特殊要求可以在此添加）
  // 例如：package 模式需要 driverPath 存在
  if (req.installMode === 'package' && !req.driverPath) {
    console.warn('[InstallService] package 模式建议提供 driverPath')
  }
}

/**
 * 格式化错误信息为用户友好的文本
 */
function formatErrorMessage(error: any, rawStdout?: string): string {
  if (typeof error === 'string') {
    // 如果是简单字符串，直接返回
    return error
  }
  
  if (error && error.message) {
    // 如果有 message 字段，优先使用
    return error.message
  }
  
  // 如果有原始 stdout，提取关键错误信息
  if (rawStdout) {
    // 避免直接返回大段 stdout，提取关键行
    const lines = rawStdout.split('\n').filter(line => 
      line.includes('错误') || 
      line.includes('失败') || 
      line.includes('Error') ||
      line.includes('Failed')
    )
    if (lines.length > 0) {
      return lines.slice(0, 3).join('; ')
    }
  }
  
  return '安装失败，详细信息请查看日志'
}

/**
 * 提交安装请求
 */
export async function submitInstall(req: InstallRequest): Promise<InstallResponse> {
  console.log('[InstallService] submitInstall called', {
    name: req.name,
    driverKey: req.driverKey,
    installMode: req.installMode,
    dryRun: req.dryRun
  })
  
  try {
    // 1. 参数校验
    validateInstallRequest(req)
    
    // 2. 准备后端参数（v2.0.0+ 使用 driverKey）
    const payload = {
      name: req.name,
      path: req.path,
      driverKey: req.driverKey || null,  // v2.0.0+：关键参数
      driverPath: req.driverPath || null,  // 向后兼容
      model: req.model || null,
      driverInstallPolicy: req.driverInstallPolicy || 'always',
      installMode: req.installMode,
      dryRun: req.dryRun ?? false
    }
    
    console.log('[InstallService] invoke install_printer', payload)
    
    // 3. 调用后端
    const result = await invoke('install_printer', payload) as any
    
    console.log('[InstallService] install_printer result', {
      success: result.success,
      jobId: result.jobId || result.job_id,
      method: result.method,
      message: result.message
    })
    
    // 4. 统一解析结果
    const response: InstallResponse = {
      success: result.success ?? false,
      jobId: result.jobId || result.job_id, // 兼容两种命名
      message: result.message || (result.success ? '安装完成' : '安装失败'),
      method: result.method,
      effectiveDryRun: result.effectiveDryRun,
      stdout: result.stdout,
      stderr: result.stderr,
      details: result
    }
    
    // 5. 如果失败，格式化错误信息
    if (!response.success) {
      response.message = formatErrorMessage(result, result.stdout)
    }
    
    return response
    
  } catch (error: any) {
    console.error('[InstallService] submitInstall error', error)
    
    // 参数校验错误直接返回
    if (error instanceof InstallValidationError) {
      return {
        success: false,
        message: error.message,
        details: { validationError: true }
      }
    }
    
    // 后端错误映射
    return {
      success: false,
      message: formatErrorMessage(error),
      details: { error: error.toString() }
    }
  }
}

/**
 * 辅助函数：确保 store 设置了活动任务
 */
export function ensureActiveJob(store: any, jobId: string | undefined, printerName: string): void {
  if (jobId) {
    store.ensureJob(jobId, printerName)
    if (!store.activeJobId) {
      store.setActiveJob(jobId)
    }
    console.log('[InstallService] ensureActiveJob', { jobId, printerName })
  } else {
    console.warn('[InstallService] ensureActiveJob: no jobId provided')
  }
}
