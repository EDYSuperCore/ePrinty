<template>
  <div class="flex items-center justify-between p-4 bg-white rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors">
    <div class="flex-1">
      <div class="flex items-center space-x-3">
        <!-- 打印机图标 -->
        <div class="flex-shrink-0 bg-gray-100 rounded-md p-2">
          <svg class="w-5 h-5 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
          </svg>
        </div>

        <!-- 打印机信息 -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center space-x-2 mb-1">
            <h3 class="text-sm font-semibold text-gray-900 truncate">{{ printer.name }}</h3>
            <!-- 已安装标识 -->
            <span v-if="isInstalled" class="flex-shrink-0 inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-700">
              <svg class="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
              </svg>
              已安装
            </span>
          </div>
          <!-- 打印机型号 -->
          <p v-if="printer.model" class="text-xs text-gray-600 truncate mb-0.5">{{ printer.model }}</p>
          <!-- 打印机路径（IP地址） -->
          <p class="text-xs text-gray-500 truncate">{{ printer.path }}</p>
        </div>
      </div>
    </div>

    <!-- 安装按钮 - 基于 detectState 显示 -->
    <div class="flex-shrink-0 ml-4">
      <!-- detecting: 检测中 -->
      <div v-if="detectState === 'detecting'" class="px-4 py-1.5 text-xs font-medium text-gray-500 flex items-center space-x-1.5">
        <svg class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <span>检测中...</span>
      </div>
      
      <!-- unknown: 状态未知，显示重试按钮 -->
      <button
        v-else-if="detectState === 'unknown'"
        @click="handleRetryDetect"
        class="px-4 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors flex items-center space-x-1.5"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        <span>重试检测</span>
      </button>
      
      <!-- not_installed: 未安装，显示安装按钮组 -->
      <div v-else-if="detectState === 'not_installed'" class="install-actions">
        <div class="flex items-stretch gap-0">
          <!-- 主安装按钮 -->
          <button
            @click="handleInstall"
            :disabled="installing"
            class="px-4 py-1.5 text-xs font-medium text-white bg-gray-900 hover:bg-gray-800 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-l-md transition-colors flex items-center space-x-1.5"
          >
            <svg v-if="installing" class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <span>{{ installing ? '安装中...' : '安装' }}</span>
          </button>
          <!-- 下拉箭头按钮 -->
          <div class="relative flex" v-click-outside="closeInstallModeMenu">
            <button
              ref="installModeButton"
              @click.stop="toggleInstallModeMenu"
              :disabled="installing"
              class="install-caret px-2 text-xs font-medium text-white bg-gray-900 hover:bg-gray-800 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-r-md border-l border-gray-700 transition-colors flex items-center justify-center"
              :title="`当前安装方式：${getInstallModeLabel(installMode)}`"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            <!-- 安装方式下拉菜单 -->
            <Teleport to="body">
              <div
                v-if="showInstallModeMenu"
                :style="installModeMenuStyle"
                class="fixed w-56 bg-white rounded-md shadow-lg border border-gray-200 z-[9999]"
              >
                <div class="py-1">
                  <button
                    v-for="option in installModeOptions"
                    :key="option.value"
                    @click.stop="selectInstallMode(option.value)"
                    :disabled="option.disabled"
                    :class="[
                      'w-full px-4 py-2 text-left text-sm transition-colors flex items-center justify-between',
                      option.disabled
                        ? 'text-gray-400 cursor-not-allowed bg-gray-50'
                        : installMode === option.value
                        ? 'bg-gray-100 text-gray-900 font-medium'
                        : 'text-gray-700 hover:bg-gray-50'
                    ]"
                    :title="option.disabled ? option.hint : ''"
                  >
                    <span>{{ option.label }}</span>
                    <div class="flex items-center space-x-2">
                      <span v-if="option.disabled" class="text-xs text-gray-400">
                        {{ option.hint }}
                      </span>
                      <svg v-if="installMode === option.value && !option.disabled" class="w-4 h-4 text-gray-600" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                      </svg>
                    </div>
                  </button>
                </div>
              </div>
            </Teleport>
          </div>
        </div>
        <!-- 安装方式提示文字 -->
        <div class="install-mode-hint">
          安装方式：{{ getInstallModeLabel(installMode) }}
        </div>
      </div>
      
      <!-- installed: 已安装，显示已就绪 + 三个点菜单 -->
      <div v-else-if="detectState === 'installed' || isInstalled" class="flex items-center space-x-2">
        <span class="px-3 py-1.5 text-xs font-medium text-gray-600">已就绪</span>
        <!-- 三个点菜单 -->
        <div class="relative" v-click-outside="closeMenu">
          <button
            ref="menuButton"
            @click.stop="toggleMenu"
            :disabled="reinstalling || installing"
            class="p-1.5 text-gray-400 hover:text-gray-600 disabled:text-gray-300 disabled:cursor-not-allowed rounded transition-colors"
            title="更多操作"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
            </svg>
          </button>
          <!-- 下拉菜单 - 使用 fixed 定位避免被父容器遮挡 -->
          <Teleport to="body">
            <div
              v-if="showMenu"
              :style="menuStyle"
              class="fixed w-48 bg-white rounded-md shadow-lg border border-gray-200 z-[9999]"
            >
            <div class="py-1">
              <button
                @click.stop="handlePrintTestPage"
                :disabled="reinstalling || installing"
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed flex items-center space-x-2"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
                </svg>
                <span>打印测试页</span>
              </button>
              <button
                @click.stop="handleReinstall"
                :disabled="reinstalling || installing"
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed flex items-center space-x-2"
              >
                <svg v-if="reinstalling" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>{{ reinstalling ? '处理中...' : '重新安装（不推荐）' }}</span>
              </button>
            </div>
            <div class="border-t border-gray-200 px-4 py-2">
              <p class="text-xs text-gray-500">高级操作，可能影响系统打印设置。若不清楚含义请勿使用。</p>
            </div>
          </div>
          </Teleport>
        </div>
      </div>
      
      <!-- error: 检测失败 -->
      <div v-else-if="detectState === 'error'" class="px-4 py-1.5 text-xs font-medium text-red-600 flex items-center space-x-1.5">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
        </svg>
        <span>检测失败</span>
      </div>
      
      <!-- empty: 系统无打印机，显示安装按钮 -->
      <div v-else-if="detectState === 'empty'" class="flex items-center space-x-2">
        <span class="px-3 py-1.5 text-xs font-medium text-gray-600">未安装</span>
        <div class="install-actions flex gap-2">
          <button
            @click="handleInstall"
            :disabled="installing"
            class="px-4 py-1.5 text-xs font-medium text-white bg-gray-900 hover:bg-gray-800 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-md transition-colors flex items-center space-x-1.5"
          >
            <svg v-if="installing" class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <span>{{ installing ? '安装中...' : '安装' }}</span>
          </button>
          <button
            @click="handleRetryDetect"
            class="px-4 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors flex items-center space-x-1.5"
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <span>重新检测</span>
          </button>
        </div>
      </div>
      
      <!-- cups_error: CUPS 服务未运行 -->
      <div v-else-if="detectState === 'cups_error'" class="px-4 py-1.5 text-xs font-medium text-amber-600 flex items-center space-x-1.5 group relative">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
        </svg>
        <span>打印服务异常</span>
        <!-- 悬停提示 -->
        <div class="absolute left-0 top-full mt-1 hidden group-hover:block bg-gray-900 text-white text-xs px-3 py-2 rounded whitespace-nowrap z-10">
          macOS 打印服务未运行，请尝试：<br/>
          1. 打开"系统设置 → 打印机与扫描仪"<br/>
          2. 或重启系统后重试
        </div>
      </div>
      
      <!-- 降级：如果没有 detectState，使用旧的 isInstalled 逻辑 -->
      <template v-else>
        <div v-if="!isInstalled" class="install-actions">
          <div class="flex items-stretch gap-0">
            <!-- 主安装按钮 -->
            <button
              @click="handleInstall"
              :disabled="installing"
              class="px-4 py-1.5 text-xs font-medium text-white bg-gray-900 hover:bg-gray-800 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-l-md transition-colors flex items-center space-x-1.5"
            >
              <svg v-if="installing" class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>{{ installing ? '安装中...' : '安装' }}</span>
            </button>
            <!-- 下拉箭头按钮 -->
            <div class="relative flex" v-click-outside="closeInstallModeMenu">
              <button
                ref="installModeButtonFallback"
                @click.stop="toggleInstallModeMenu"
                :disabled="installing"
                class="install-caret px-2 text-xs font-medium text-white bg-gray-900 hover:bg-gray-800 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-r-md border-l border-gray-700 transition-colors flex items-center justify-center"
                :title="`当前安装方式：${getInstallModeLabel(installMode)}`"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              <!-- 安装方式下拉菜单 -->
              <Teleport to="body">
                <div
                  v-if="showInstallModeMenu"
                  :style="installModeMenuStyle"
                  class="fixed w-56 bg-white rounded-md shadow-lg border border-gray-200 z-[9999]"
                >
                  <div class="py-1">
                    <button
                      v-for="option in installModeOptions"
                      :key="option.value"
                      @click.stop="selectInstallMode(option.value)"
                      :class="[
                        'w-full px-4 py-2 text-left text-sm transition-colors flex items-center justify-between',
                        installMode === option.value
                          ? 'bg-gray-100 text-gray-900 font-medium'
                          : 'text-gray-700 hover:bg-gray-50'
                      ]"
                    >
                      <span>{{ option.label }}</span>
                      <svg v-if="installMode === option.value" class="w-4 h-4 text-gray-600" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                      </svg>
                    </button>
                  </div>
                </div>
              </Teleport>
            </div>
          </div>
          <!-- 安装方式提示文字 -->
          <div class="install-mode-hint">
            安装方式：{{ getInstallModeLabel(installMode) }}
          </div>
        </div>
        <div v-else class="flex items-center space-x-2">
          <span class="px-3 py-1.5 text-xs font-medium text-gray-600">已就绪</span>
          <!-- 三个点菜单 -->
          <div class="relative" v-click-outside="closeMenu">
            <button
              ref="menuButtonFallback"
              @click.stop="toggleMenu"
              :disabled="reinstalling || installing"
              class="p-1.5 text-gray-400 hover:text-gray-600 disabled:text-gray-300 disabled:cursor-not-allowed rounded transition-colors"
              title="更多操作"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
              </svg>
            </button>
            <!-- 下拉菜单 - 使用 fixed 定位避免被父容器遮挡 -->
            <Teleport to="body">
              <div
                v-if="showMenu"
                :style="menuStyle"
                class="fixed w-48 bg-white rounded-md shadow-lg border border-gray-200 z-[9999]"
              >
                <div class="py-1">
                  <button
                    @click.stop="handlePrintTestPage"
                    :disabled="reinstalling || installing"
                    class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed flex items-center space-x-2"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
                    </svg>
                    <span>打印测试页</span>
                  </button>
                  <button
                    @click.stop="handleReinstall"
                    :disabled="reinstalling || installing"
                    class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed flex items-center space-x-2"
                  >
                    <svg v-if="reinstalling" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    <span>{{ reinstalling ? '处理中...' : '重新安装（不推荐）' }}</span>
                  </button>
                </div>
                <div class="border-t border-gray-200 px-4 py-2">
                  <p class="text-xs text-gray-500">高级操作，可能影响系统打印设置。若不清楚含义请勿使用。</p>
                </div>
              </div>
            </Teleport>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script>
export default {
  name: 'PrinterItem',
  props: {
    printer: {
      type: Object,
      required: true
    },
    isInstalled: {
      type: Boolean,
      default: false
    },
    detectState: {
      type: String,
      default: 'unknown', // 'detecting' | 'installed' | 'not_installed' | 'unknown' | 'error' | 'empty' | 'cups_error'
      validator: (value) => ['detecting', 'installed', 'not_installed', 'unknown', 'error', 'empty', 'cups_error'].includes(value)
    },
    installing: {
      type: Boolean,
      default: false
    },
    reinstalling: {
      type: Boolean,
      default: false
    },
    installMode: {
      type: String,
      default: 'auto'
    },
  },
  data() {
    return {
      showMenu: false,
      menuStyle: {
        top: '0px',
        right: '0px'
      },
      showInstallModeMenu: false,
      installModeMenuStyle: {
        top: '0px',
        right: '0px'
      },
      // 安装方式选项
      installModeOptions: [
        { value: 'auto', label: '自动兼容（推荐）', disabled: false },
        { value: 'package', label: '驱动包安装（推荐）', disabled: false },
        { value: 'installer', label: '厂商安装程序', disabled: true, hint: '暂未开放' },
        { value: 'ipp', label: '免驱打印（系统通用）', disabled: true, hint: '暂未开放' },
        { value: 'legacy_inf', label: '传统 INF 安装（老型号）', disabled: true, hint: '暂未开放' }
      ],
      // 阶段顺序定义
      phasesOrder: [
        { key: 'download', label: '下载驱动包' },
        { key: 'verify', label: '校验驱动包' },
        { key: 'extract', label: '解压并合并' },
        { key: 'stageDriver', label: '导入 DriverStore' },
        { key: 'registerDriver', label: '注册打印驱动' },
        { key: 'ensurePort', label: '创建/校验端口' },
        { key: 'ensureQueue', label: '创建/校验队列' },
        { key: 'finalVerify', label: '最终验证' }
      ]
    }
  },
  watch: {
    installing(newVal, oldVal) {
      // [UI][InstallButton] 插桩日志 - 监听 installing 状态变化
      console.log(`[UI][InstallButton] id=${this.printer.name} state=${this.isInstalled ? 'installed' : 'idle'} installing=${newVal} (changed from ${oldVal})`)
    },
    isInstalled(newVal, oldVal) {
      // [UI][InstallButton] 插桩日志 - 监听 isInstalled 状态变化
      console.log(`[UI][InstallButton] id=${this.printer.name} state=${newVal ? 'installed' : 'idle'} installing=${this.installing} (changed from ${oldVal ? 'installed' : 'idle'})`)
    }
  },
  methods: {
    handleInstall() {
      // [UI][InstallClick] 插桩日志
      console.log(`[UI][InstallClick] id=${this.printer.name} before=installing=${this.installing} state=${this.isInstalled ? 'installed' : 'idle'} detectState=${this.detectState} installMode=${this.installMode}`)
      console.log('PrinterItem: 点击安装按钮', this.printer, '安装方式:', this.installMode)
      // 直接触发 install 事件，安装状态由父组件 App.vue 统一管理
      this.$emit('install', this.printer)
    },
    toggleInstallModeMenu() {
      if (!this.showInstallModeMenu) {
        // 打开菜单时计算位置
        this.$nextTick(() => {
          const button = this.$refs.installModeButton || this.$refs.installModeButtonFallback
          if (button) {
            const buttonRect = button.getBoundingClientRect()
            this.installModeMenuStyle = {
              top: `${buttonRect.bottom + 4}px`,
              right: `${window.innerWidth - buttonRect.right}px`
            }
          }
        })
      }
      this.showInstallModeMenu = !this.showInstallModeMenu
    },
    closeInstallModeMenu() {
      this.showInstallModeMenu = false
    },
    selectInstallMode(mode) {
      // 检查是否是禁用项
      const option = this.installModeOptions.find(opt => opt.value === mode)
      if (option && option.disabled) {
        console.warn(`[InstallMode] ${mode} is disabled, ignoring selection`)
        return
      }
      
      this.closeInstallModeMenu()
      // 使用 nextTick 确保菜单关闭后再触发事件
      this.$nextTick(() => {
        this.$emit('set-install-mode', mode)
      })
    },
    getInstallModeLabel(mode) {
      const option = this.installModeOptions.find(opt => opt.value === mode)
      return option ? option.label : '自动兼容（推荐）'
    },
    handleRetryDetect() {
      console.log(`[UI][RetryDetect] id=${this.printer.name}`)
      // 触发重试检测事件
      this.$emit('retry-detect')
    },
    toggleMenu() {
      if (!this.showMenu) {
        // 打开菜单时计算位置
        this.$nextTick(() => {
          // 尝试使用 menuButton（新逻辑）或 menuButtonFallback（降级逻辑）
          const button = this.$refs.menuButton || this.$refs.menuButtonFallback
          if (button) {
            const buttonRect = button.getBoundingClientRect()
            this.menuStyle = {
              top: `${buttonRect.bottom + 4}px`,
              right: `${window.innerWidth - buttonRect.right}px`
            }
          }
        })
      }
      this.showMenu = !this.showMenu
    },
    closeMenu() {
      this.showMenu = false
    },
    handleReinstall() {
      this.closeMenu()
      // 使用 nextTick 确保菜单关闭后再触发事件
      this.$nextTick(() => {
        this.$emit('reinstall', this.printer)
      })
    },
    handlePrintTestPage() {
      this.closeMenu()
      // 使用 nextTick 确保菜单关闭后再触发事件
      this.$nextTick(() => {
        this.$emit('print-test-page', this.printer)
      })
    },
    handleRemove() {
      this.closeMenu()
      // 使用 nextTick 确保菜单关闭后再触发事件
      this.$nextTick(() => {
        this.$emit('remove', this.printer)
      })
    },
    formatBytes(bytes) {
      if (bytes === 0) return '0 B'
      const k = 1024
      const sizes = ['B', 'KB', 'MB', 'GB']
      const i = Math.floor(Math.log(bytes) / Math.log(k))
      return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
    },
    // 获取下载百分比
    getDownloadPercent(phase) {
      if (!phase) return null
      if (phase.percent !== null && phase.percent !== undefined) {
        return phase.percent
      }
      if (phase.bytesDownloaded && phase.bytesTotal && phase.bytesTotal > 0) {
        return Math.floor((phase.bytesDownloaded / phase.bytesTotal) * 100)
      }
      return null
    },
    // 判断是否应该显示该阶段
    shouldShowPhase(phases, phaseKey) {
      if (!phases || !phases[phaseKey]) return false
      const phase = phases[phaseKey]
      // 显示非 pending 状态的阶段，或者已经完成/失败的阶段
      return phase.state !== 'pending' || phase.updatedAt > 0
    },
    // 获取阶段状态
    getPhaseState(phases, phaseKey) {
      if (!phases || !phases[phaseKey]) return 'pending'
      return phases[phaseKey].state || 'pending'
    },
    // 获取阶段消息
    getPhaseMessage(phases, phaseKey) {
      if (!phases || !phases[phaseKey]) return null
      return phases[phaseKey].message || null
    }
  },
  directives: {
    'click-outside': {
      mounted(el, binding) {
        el.clickOutsideEvent = (event) => {
          if (!(el === event.target || el.contains(event.target))) {
            binding.value()
          }
        }
        document.addEventListener('click', el.clickOutsideEvent)
      },
      unmounted(el) {
        document.removeEventListener('click', el.clickOutsideEvent)
      }
    }
  }
}
</script>

<style scoped>
.install-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}

.install-mode-hint {
  font-size: 12px;
  color: #6b7280; /* text-gray-500 */
  text-align: right;
  margin-top: 2px;
}

.install-caret {
  min-width: 36px;
  padding-left: 8px;
  padding-right: 8px;
  /* 确保与主按钮高度一致 */
  padding-top: 6px;
  padding-bottom: 6px;
}
</style>
