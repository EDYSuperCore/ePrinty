/**
 * 打印机匹配工具
 * 职责：
 * - 打印机名称规范化
 * - 匹配规则（名称、URI、deviceUri）
 */

/**
 * 规范化打印机名称
 */
export function normalizePrinterName(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/\s+/g, ' ') // 多个空格合并为一个
    .replace(/\u3000/g, ' ') // 全角空格转半角
}

/**
 * 检查打印机名称是否匹配
 */
export function printerNameMatches(configName: string, installedName: string): boolean {
  const normalized1 = normalizePrinterName(configName)
  const normalized2 = normalizePrinterName(installedName)

  // 精确匹配
  if (normalized1 === normalized2) {
    return true
  }

  return false
}

/**
 * 规范化 deviceUri（用于 URI 匹配）
 */
export function normalizeDeviceUri(uri: string | null | undefined): string | null {
  if (!uri) return null

  const normalized = uri
    .trim()
    .toLowerCase()
    .replace(/\s+/g, '')
    .replace(/\u3000/g, '')

  return normalized.length > 0 ? normalized : null
}

/**
 * 根据 path 构建 deviceUri（用于匹配）
 * Windows: socket://ip:port 或 http://ip:port
 * macOS: ipp://ip:port/ipp/... 或 https://ip:port/ipp/...
 */
export function buildDeviceUriFromPath(path: string): string | null {
  if (!path) return null

  const normalized = path
    .trim()
    .toLowerCase()
    .replace(/\s+/g, '')
    .replace(/\u3000/g, '')

  // 检查是否以常见协议开头
  if (
    normalized.startsWith('socket://') ||
    normalized.startsWith('http://') ||
    normalized.startsWith('ipp://') ||
    normalized.startsWith('https://') ||
    normalized.startsWith('lpd://') ||
    normalized.startsWith('snmp://')
  ) {
    return normalized
  }

  return null
}

/**
 * 找到配置中的打印机信息
 */
export function findPrinterConfigByName(
  config: any,
  printerName: string
): any | null {
  if (!config || !config.cities) return null

  for (const city of config.cities) {
    if (!city.areas) continue
    for (const area of city.areas) {
      if (!area.printers) continue
      const printer = area.printers.find((p: any) => p.name === printerName)
      if (printer) return printer
    }
  }

  return null
}
