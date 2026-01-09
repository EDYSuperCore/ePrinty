// 应用设置管理模块
// 提供统一的设置读写接口，用于持久化高级选项

const SETTINGS_KEY = 'eprinty.settings.v1'
const LEGACY_DRIVER_POLICY_KEY = 'driverInstallPolicy' // 旧版本的 key，用于迁移

export type DriverInstallStrategy = 'always_install_inf' | 'skip_if_driver_exists'

export interface AppSettings {
  remove_port: boolean
  remove_driver: boolean
  driver_install_strategy: DriverInstallStrategy
}

const DEFAULT_SETTINGS: AppSettings = {
  remove_port: false,
  remove_driver: false,
  driver_install_strategy: 'always_install_inf' // 默认值：总是安装/更新 INF 驱动（稳定）
}

/**
 * 迁移旧版本的驱动安装策略设置
 * 从独立的 localStorage key 迁移到统一 settings
 */
function migrateLegacyDriverPolicy(): DriverInstallStrategy | null {
  try {
    const legacyValue = localStorage.getItem(LEGACY_DRIVER_POLICY_KEY)
    if (!legacyValue) {
      return null
    }

    // 映射旧值到新值
    if (legacyValue === 'always') {
      return 'always_install_inf'
    } else if (legacyValue === 'reuse_if_installed') {
      return 'skip_if_driver_exists'
    }
    
    // 非法值，返回 null
    return null
  } catch (error) {
    console.warn('[Settings] Failed to migrate legacy driver policy:', error)
    return null
  }
}

/**
 * 验证驱动安装策略值是否合法
 */
function validateDriverInstallStrategy(value: any): DriverInstallStrategy {
  if (value === 'always_install_inf' || value === 'skip_if_driver_exists') {
    return value
  }
  // 非法值，返回默认值
  console.warn(`[Settings] Invalid driver_install_strategy: ${value}, using default`)
  return DEFAULT_SETTINGS.driver_install_strategy
}

/**
 * 获取应用设置
 * 如果 localStorage 中没有设置或格式不完整，返回默认值
 * 支持从旧版本迁移数据
 */
export function getAppSettings(): AppSettings {
  try {
    const stored = localStorage.getItem(SETTINGS_KEY)
    let parsed: Partial<AppSettings> = {}
    let needsMigration = false

    if (stored) {
      try {
        parsed = JSON.parse(stored) as Partial<AppSettings>
      } catch (error) {
        console.warn('[Settings] Failed to parse stored settings, using defaults:', error)
        parsed = {}
      }
    }

    // 检查是否需要迁移旧版本的驱动安装策略
    if (!parsed.driver_install_strategy) {
      const migratedValue = migrateLegacyDriverPolicy()
      if (migratedValue) {
        parsed.driver_install_strategy = migratedValue
        needsMigration = true
        console.log('[Settings] Migrated legacy driver policy:', migratedValue)
      }
    }

    // 确保所有字段都存在，缺失的字段使用默认值
    const result: AppSettings = {
      remove_port: parsed.remove_port ?? DEFAULT_SETTINGS.remove_port,
      remove_driver: parsed.remove_driver ?? DEFAULT_SETTINGS.remove_driver,
      driver_install_strategy: validateDriverInstallStrategy(
        parsed.driver_install_strategy ?? DEFAULT_SETTINGS.driver_install_strategy
      )
    }

    // 如果进行了迁移或补全了缺失字段，立即保存
    if (needsMigration || !stored || !parsed.driver_install_strategy) {
      setAppSettingsFull(result)
    }

    return result
  } catch (error) {
    console.error('[Settings] Failed to load settings:', error)
    return { ...DEFAULT_SETTINGS }
  }
}

/**
 * 设置应用设置（部分更新）
 * @param partial 要更新的设置项（可以是部分字段）
 * @returns 返回合并后的最终设置值
 */
export function setAppSettings(partial: Partial<AppSettings>): AppSettings {
  try {
    const current = getAppSettings()
    
    // 如果更新了 driver_install_strategy，需要验证
    if (partial.driver_install_strategy !== undefined) {
      partial.driver_install_strategy = validateDriverInstallStrategy(partial.driver_install_strategy)
    }
    
    const updated = { ...current, ...partial }
    
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(updated))
    console.log('[Settings] Settings updated:', updated)
    
    return updated
  } catch (error) {
    console.error('[Settings] Failed to save settings:', error)
    return getAppSettings() // 返回当前设置
  }
}

/**
 * 设置应用设置（完整替换）
 * @param settings 完整的设置对象
 */
export function setAppSettingsFull(settings: AppSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
    console.log('[Settings] Settings saved:', settings)
  } catch (error) {
    console.error('[Settings] Failed to save settings:', error)
  }
}

/**
 * 获取危险删除选项（供后续删除/重装功能调用）
 * 这是统一的入口函数，后续删除/重装逻辑应调用此函数获取设置
 */
export function getDangerousRemoveOptions(): { remove_port: boolean; remove_driver: boolean } {
  const settings = getAppSettings()
  return {
    remove_port: settings.remove_port,
    remove_driver: settings.remove_driver
  }
}

/**
 * 获取驱动安装策略（供后续安装/重装功能调用）
 * 这是统一的入口函数，后续安装/重装逻辑应调用此函数获取设置
 */
export function getDriverInstallStrategy(): DriverInstallStrategy {
  const settings = getAppSettings()
  return settings.driver_install_strategy
}

/**
 * 重置设置为默认值
 */
export function resetAppSettings(): void {
  try {
    localStorage.removeItem(SETTINGS_KEY)
    console.log('[Settings] Settings reset to defaults')
  } catch (error) {
    console.error('[Settings] Failed to reset settings:', error)
  }
}

