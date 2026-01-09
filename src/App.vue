<template>
<div class="app-frame" @keydown="handleGlobalKeyDown" tabindex="-1" ref="appFrame">
  <div class="app-shell">
<AppTitleBar>
  <template #actions>
    <div class="flex items-center space-x-2">
          <!-- 调试模式按钮（默认隐藏，按 Ctrl+Shift+D 显示） -->
          <button
            v-if="showDebugButton"
            @click="toggleDebugWindow"
            :class="[
              'flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-all duration-200',
              showDebugWindow 
                ? 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200' 
                : 'text-gray-700 hover:bg-gray-100'
            ]"
            :title="showDebugWindow ? '隐藏调试窗口' : '显示调试窗口'"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <span>调试</span>
            <span v-if="debugLogs.length > 0" class="ml-1 px-1.5 py-0.5 text-xs bg-red-500 text-white rounded-full">
              {{ debugLogs.length }}
            </span>
          </button>
          <!-- 设置按钮 -->
          <button
            @click="openSettings"
            class="flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-lg transition-all duration-200"
            title="设置"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            <span>设置</span>
          </button>
          <!-- IT热线按钮 -->
          <button
            @click="openDingTalk"
            class="flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-lg transition-all duration-200"
            title="IT热线"
          >
            <!-- 钉钉图标 -->
            <img :src="dingtalkIcon" alt="钉钉" class="w-4 h-4 object-contain" />
            <span>IT热线</span>
          </button>
        </div>
  </template>
</AppTitleBar>
    <div class="app-content">
  <div class="flex flex-col h-full bg-gray-50" @contextmenu.prevent>
    <!-- 主体内容 -->
    <div class="flex-1 flex overflow-hidden">
      <!-- 左侧：办公区选择器 -->
      <aside class="w-64 bg-white border-r border-gray-200 flex flex-col shadow-sm">
        <div class="p-4 border-b border-gray-200 bg-white">
          <h2 class="text-xs font-semibold text-gray-500 uppercase tracking-wider">选择办公区</h2>
        </div>
        
        <!-- 加载状态 -->
        <div v-if="loading" class="flex-1 flex items-center justify-center p-4">
          <div class="text-center">
            <div class="inline-block animate-spin rounded-full h-8 w-8 border-2 border-gray-200 border-t-gray-400 mb-2"></div>
            <p class="text-xs text-gray-500">加载中...</p>
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-else-if="error" class="flex-1 flex items-center justify-center p-4">
          <div class="text-center">
            <svg class="w-8 h-8 text-red-500 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <p class="text-sm text-red-600 mb-3">{{ error }}</p>
            <button 
              @click="loadData"
              class="px-3 py-1.5 text-sm bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
            >
              重试
            </button>
          </div>
        </div>

        <!-- 办公区列表 -->
        <div v-else-if="config && config.areas && config.areas.length > 0" class="flex-1 overflow-y-auto">
          <button
            v-for="(area, index) in config.areas"
            :key="area.name"
            @click="selectArea(index)"
            :class="[
              'w-full px-4 py-3 text-left transition-all duration-150 relative group',
              selectedAreaIndex === index 
                ? 'bg-gray-100 text-gray-900' 
                : 'hover:bg-gray-50 text-gray-700'
            ]"
          >
            <div class="flex items-center justify-between">
              <span class="font-medium text-sm truncate flex-1 min-w-0">{{ area.name }}</span>
              <span :class="[
                'flex-shrink-0 text-xs font-medium px-2 py-0.5 rounded-full transition-all',
                selectedAreaIndex === index
                  ? 'bg-gray-700 text-white'
                  : 'bg-gray-200 text-gray-600'
              ]">
                {{ area.printers ? area.printers.length : 0 }}
              </span>
            </div>
          </button>
        </div>

        <!-- 空状态 -->
        <div v-else class="flex-1 flex items-center justify-center p-4">
          <p class="text-sm text-gray-500">暂无办公区</p>
        </div>
      </aside>

      <!-- 右侧：打印机列表 -->
      <main class="flex-1 overflow-y-auto px-6 py-4">
        <!-- 首次加载打印机提示（在数据加载完成前显示） -->
        <div v-if="initialLoadingPrinters" class="flex items-center justify-center h-full">
          <div class="text-center">
            <div class="inline-block animate-spin rounded-full h-12 w-12 border-2 border-gray-200 border-t-gray-600 mb-4"></div>
            <p class="text-sm font-medium text-gray-700">正在加载打印机...</p>
          </div>
        </div>

        <!-- 加载状态 -->
        <div v-else-if="loading" class="flex items-center justify-center h-full">
          <div class="text-center">
            <div class="inline-block animate-spin rounded-full h-12 w-12 border-2 border-gray-200 border-t-gray-600 mb-4"></div>
            <p class="text-sm font-medium text-gray-700">正在加载打印机配置...</p>
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-else-if="error" class="flex items-center justify-center h-full">
          <div class="bg-white border-2 border-red-200 rounded-xl p-8 max-w-md shadow-xl">
            <div class="flex items-center justify-center mb-4">
              <div class="bg-red-100 rounded-full p-4">
                <svg class="w-8 h-8 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
            </div>
            <h3 class="text-xl font-bold text-red-800 text-center mb-3">加载失败</h3>
            <p class="text-red-600 text-center mb-6">{{ error }}</p>
            <button 
              @click="loadData"
              class="w-full px-6 py-3 bg-gradient-to-r from-red-500 to-red-600 text-white font-semibold rounded-lg hover:from-red-600 hover:to-red-700 transition-all duration-200 shadow-md hover:shadow-lg transform hover:scale-105 flex items-center justify-center space-x-2"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              <span>重试</span>
            </button>
          </div>
        </div>

        <!-- 未选择办公区提示 -->
        <div v-else-if="selectedAreaIndex === null" class="flex items-center justify-center h-full">
          <div class="text-center max-w-sm">
            <div class="bg-gray-100 rounded-full w-16 h-16 flex items-center justify-center mx-auto mb-4">
              <svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
              </svg>
            </div>
            <h3 class="text-lg font-semibold text-gray-900 mb-2">请先选择办公区</h3>
            <p class="text-sm text-gray-500">从左侧列表中选择一个办公区查看打印机</p>
          </div>
        </div>

        <!-- 首次加载打印机提示（在数据加载完成前显示） -->
        <div v-else-if="initialLoadingPrinters" class="flex items-center justify-center h-full">
          <div class="text-center">
            <div class="inline-block animate-spin rounded-full h-12 w-12 border-2 border-gray-200 border-t-gray-600 mb-4"></div>
            <p class="text-sm font-medium text-gray-700">正在加载打印机...</p>
          </div>
        </div>

        <!-- 选中的办公区打印机列表 -->
        <div v-else-if="selectedArea" class="space-y-4">
          <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <!-- 办公区标题 -->
            <div class="bg-gray-50 border-b border-gray-200 px-5 py-3">
              <div class="flex items-center justify-between">
                <h2 class="text-base font-semibold text-gray-900">{{ selectedArea.name }}</h2>
                <span class="text-xs font-medium text-gray-500 bg-gray-200 px-2.5 py-1 rounded-full">
                  {{ selectedArea.printers ? selectedArea.printers.length : 0 }} 台
                </span>
              </div>
            </div>

            <!-- 打印机列表 -->
            <div class="p-4 space-y-3 bg-white">
              <PrinterItem
                v-for="printer in selectedArea.printers"
                :key="printer.name"
                :printer="printer"
                :is-installed="isInstalled(printer.name)"
                :detect-state="getPrinterDetectState(printer.name)"
                :installing="installingPrinters.has(printer.name)"
                :reinstalling="reinstallingPrinters.has(printer.name)"
                :install-mode="getInstallMode(printer)"
                @install="handleInstall"
                @retry-detect="retryDetect"
                @reinstall="handleReinstall"
                @print-test-page="handlePrintTestPage"
                @set-install-mode="(mode) => setInstallMode(printer, mode)"
              />
            </div>
          </div>
        </div>

        <!-- 空状态 -->
        <div v-else class="flex items-center justify-center h-full">
          <p class="text-gray-500">该办公区暂无打印机</p>
        </div>
      </main>
    </div>

    <!-- 底部状态栏 -->
    <footer class="bg-white border-t border-gray-200 px-5 py-2.5">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-2">
          <div :class="[
            'w-1.5 h-1.5 rounded-full',
            statusType === 'success' ? 'bg-green-500' : 
            statusType === 'error' ? 'bg-red-500' : 
            statusType === 'info' ? 'bg-gray-500' :
            'bg-gray-300'
          ]"></div>
          <span class="text-xs text-gray-500">状态:</span>
          <span :class="[
            'text-xs font-medium',
            statusType === 'success' ? 'text-green-600' : 
            statusType === 'error' ? 'text-red-600' : 
            statusType === 'info' ? 'text-gray-700' :
            'text-gray-600'
          ]">
            {{ statusMessage || '就绪' }}
          </span>
        </div>
        <button
          @click="refresh"
          class="px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-100 rounded-md transition-colors flex items-center space-x-1.5"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span>刷新</span>
        </button>
      </div>
    </footer>

    <!-- 设置对话框 -->
    <div 
      v-if="showHelp" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm overflow-y-auto"
      @click.self="showHelp = false"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-3xl w-full mx-4 my-8 overflow-hidden flex flex-col max-h-[85vh]">
        <!-- 对话框标题 -->
        <div class="bg-gray-50 border-b border-gray-200 px-6 py-4 relative z-10 flex-shrink-0">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">设置</h3>
            <button
              @click="showHelp = false"
              class="text-gray-400 hover:text-gray-600 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容：左侧Tab + 右侧内容 -->
        <div class="flex flex-1 overflow-hidden">
          <!-- 左侧Tab导航 -->
          <div class="w-48 bg-gray-50 border-r border-gray-200 flex-shrink-0 flex flex-col">
            <div class="p-2 space-y-1">
              <button
                @click="settingsTab = 'settings'"
                :class="[
                  'w-full px-4 py-3 text-left rounded-lg transition-all duration-150',
                  settingsTab === 'settings'
                    ? 'bg-white text-blue-600 shadow-sm font-medium'
                    : 'text-gray-700 hover:bg-gray-100'
                ]"
              >
                <div class="flex items-center space-x-2">
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  <span>设置</span>
                </div>
              </button>
              <button
                @click="settingsTab = 'about'"
                :class="[
                  'w-full px-4 py-3 text-left rounded-lg transition-all duration-150',
                  settingsTab === 'about'
                    ? 'bg-white text-blue-600 shadow-sm font-medium'
                    : 'text-gray-700 hover:bg-gray-100'
                ]"
              >
                <div class="flex items-center space-x-2">
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span>关于</span>
                </div>
              </button>
              <button
                @click="settingsTab = 'recommend'"
                :class="[
                  'w-full px-4 py-3 text-left rounded-lg transition-all duration-150',
                  settingsTab === 'recommend'
                    ? 'bg-white text-blue-600 shadow-sm font-medium'
                    : 'text-gray-700 hover:bg-gray-100'
                ]"
              >
                <div class="flex items-center space-x-2">
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                  </svg>
                  <span>推荐</span>
                </div>
              </button>
            </div>
          </div>

          <!-- 右侧内容区域 -->
          <div class="flex-1 overflow-y-auto">
            <!-- Tab 1：设置 -->
            <div v-if="settingsTab === 'settings'" class="p-6">
            
            <!-- 风险提示 -->
            <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4 mb-6">
              <div class="flex items-start space-x-3">
                <svg class="w-5 h-5 text-yellow-600 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <div class="flex-1">
                  <p class="text-sm font-medium text-yellow-800 mb-1">
                    如果你不知道以下选项代表什么含义，请不要随便修改。
                  </p>
                  <p class="text-xs text-yellow-700">
                    这些选项会影响系统打印机配置，错误设置可能导致打印机无法正常使用。
                  </p>
                </div>
              </div>
            </div>

            <!-- 高级选项 -->
            <div class="space-y-6">

              <!-- 设置项 1：测试模式（dryRun） -->
              <div class="border border-gray-200 rounded-lg p-4">
                <div class="flex-1">
                  <div class="flex items-center space-x-2 mb-2">
                    <h5 class="text-sm font-medium text-gray-900">测试模式（dryRun）</h5>
                    <span class="px-2 py-0.5 text-xs font-medium bg-yellow-100 text-yellow-700 rounded">推荐</span>
                  </div>
                  <p class="text-xs text-gray-600 mb-3">
                    开启后，安装流程将仅模拟执行，不会对系统进行任何真实修改。关闭后，将执行真实安装（当前版本尚未接入真实安装逻辑）。
                  </p>
                  <label class="flex items-center space-x-2 cursor-pointer">
                    <input
                      type="checkbox"
                      v-model="dryRun"
                      @change="onDryRunChange"
                      class="w-4 h-4 text-blue-600 focus:ring-blue-500 rounded"
                    />
                    <span class="text-sm text-gray-700">启用测试模式（默认开启）</span>
                  </label>
                </div>
              </div>

              <!-- 设置项 3：驱动安装策略 -->
              <div class="border border-gray-200 rounded-lg p-4">
                <div class="flex-1">
                  <div class="flex items-center space-x-2 mb-2">
                    <h5 class="text-sm font-medium text-gray-900">驱动安装策略</h5>
                    <span class="px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-700 rounded">高级</span>
                  </div>
                  <p class="text-xs text-gray-600 mb-3">
                    切换策略可能影响安装一致性与速度，请在 IT 指导下修改。
                  </p>
                  <div class="space-y-2 mt-3">
                    <label class="flex items-center space-x-2 cursor-pointer">
                      <input
                        type="radio"
                        v-model="appSettings.driver_install_strategy"
                        value="always_install_inf"
                        @change="onDriverStrategyChange('always_install_inf')"
                        class="w-4 h-4 text-blue-600 focus:ring-blue-500"
                      />
                      <span class="text-sm text-gray-700">总是安装/更新 INF 驱动（稳定）</span>
                    </label>
                    <label class="flex items-center space-x-2 cursor-pointer">
                      <input
                        type="radio"
                        v-model="appSettings.driver_install_strategy"
                        value="skip_if_driver_exists"
                        @change="onDriverStrategyChange('skip_if_driver_exists')"
                        class="w-4 h-4 text-blue-600 focus:ring-blue-500"
                      />
                      <span class="text-sm text-gray-700">若系统已存在驱动则跳过 INF（更快，可能版本不一致）</span>
                    </label>
                  </div>
                </div>
              </div>
            </div>
            </div>

            <!-- Tab 2：关于 -->
            <div v-if="settingsTab === 'about'" class="p-6">
            
            <div class="flex items-center space-x-4 mb-6">
              <div class="flex-shrink-0 w-16 h-16 bg-gray-50 rounded-xl flex items-center justify-center overflow-hidden shadow-sm">
                <img src="/icon.png" alt="ePrinty" class="w-full h-full object-contain p-2" />
              </div>
              <div>
                <h4 class="text-xl font-semibold text-gray-900">ePrinty</h4>
                <p class="text-sm text-gray-500 mt-1">让打印这件事，简单一点</p>
              </div>
            </div>

            <div class="space-y-4 border-t border-gray-200 pt-4">
              <div class="flex items-start space-x-3">
                <svg class="w-5 h-5 text-gray-400 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                </svg>
                <div class="flex-1">
                  <p class="text-xs text-gray-500 mb-0.5">版本号</p>
                  <p class="text-sm font-medium text-gray-900">{{ version }}</p>
                </div>
              </div>

              <div class="flex items-start space-x-3">
                <svg class="w-5 h-5 text-gray-400 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                <div class="flex-1">
                  <p class="text-xs text-gray-500 mb-0.5">作者</p>
                  <p class="text-sm font-medium text-gray-900">易点云 研发中心核心业务组</p>
                </div>
              </div>
            </div>
            </div>

            <!-- Tab 3：推荐 -->
            <div v-if="settingsTab === 'recommend'" class="p-6">
              <h4 class="text-base font-semibold text-gray-900 mb-4">推荐</h4>
              
              <!-- 作者的其他作品 -->
              <div v-if="otherProducts && otherProducts.length > 0" class="space-y-3">
                <div
                  v-for="product in otherProducts"
                  :key="product.name"
                  class="flex items-start space-x-3 p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
                >
                  <!-- 产品图标 -->
                  <div v-if="product.icon" class="flex-shrink-0 w-12 h-12 bg-gray-50 rounded-lg flex items-center justify-center overflow-hidden">
                    <img
                      :src="product.icon"
                      :alt="product.name"
                      class="w-full h-full object-contain"
                    />
                  </div>
                  <!-- 产品信息 -->
                  <div class="flex-1 min-w-0 flex items-start justify-between">
                    <div class="flex-1 min-w-0">
                      <p class="text-sm font-medium text-gray-900 mb-1">{{ product.name }}</p>
                      <p class="text-xs text-gray-500 mb-2">{{ product.description }}</p>
                    </div>
                    <button
                      @click="openProductUrl(product.url)"
                      class="ml-3 px-3 py-1.5 text-xs font-medium text-blue-600 hover:text-blue-700 hover:bg-blue-50 rounded-md transition-colors flex-shrink-0"
                    >
                      了解更多
                    </button>
                  </div>
                </div>
              </div>
              <div v-else class="text-center py-8 text-gray-500 text-sm">
                暂无推荐内容
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 配置更新提示对话框 -->
    <div 
      v-if="showUpdateDialog" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="cancelUpdate"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        <!-- 对话框标题 -->
        <div class="bg-gray-50 border-b border-gray-200 px-6 py-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">配置更新可用</h3>
            <button
              @click="cancelUpdate"
              class="text-gray-400 hover:text-gray-600 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6">
          <div class="flex items-center space-x-4 mb-4">
            <div class="bg-blue-100 rounded-full p-3">
              <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </div>
            <div class="flex-1">
              <h4 class="text-base font-semibold text-gray-900 mb-1">检测到远程配置更新</h4>
              <p class="text-sm text-gray-600">是否下载并应用最新配置？</p>
            </div>
          </div>

          <div class="bg-gray-50 rounded-lg p-4 mb-4">
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500">本地版本</span>
                <span class="text-sm font-medium text-gray-700">{{ localVersion }}</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500">远程版本</span>
                <span class="text-sm font-medium text-blue-600">{{ remoteVersion }}</span>
              </div>
            </div>
          </div>

          <p class="text-xs text-gray-500 mb-4">更新后会自动刷新打印机列表</p>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4">
          <div class="flex items-center space-x-3">
            <button
              @click="cancelUpdate"
              class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
            >
              取消
            </button>
            <button
              @click="confirmUpdate"
              class="flex-1 px-4 py-2 text-sm font-medium text-white bg-gray-900 hover:bg-gray-800 rounded-md transition-colors"
            >
              更新
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 安装进度对话框 -->
    <div 
      v-if="showInstallProgress" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="handleInstallProgressBackgroundClick"
    >
      <div 
        class="bg-white rounded-xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden flex flex-col max-h-[90vh]"
        @click.stop
      >
        <!-- 对话框标题 -->
        <div class="bg-gray-50 border-b border-gray-200 px-6 py-4 flex-shrink-0">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">正在安装打印机</h3>
            <button
              v-if="installProgress.currentStep >= installProgress.steps.length"
              @click="closeInstallProgress"
              class="text-gray-400 hover:text-gray-600 transition-colors"
              title="关闭"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6 flex-1 overflow-y-auto min-h-0">
          <!-- 打印机名称 -->
          <div class="mb-4 flex-shrink-0">
            <h4 class="text-base font-medium text-gray-900 mb-2">{{ installProgress.printerName }}</h4>
            <p v-if="installProgress.printerPath" class="text-xs text-gray-500">{{ installProgress.printerPath }}</p>
          </div>

          <!-- 进度步骤列表 -->
          <div class="space-y-3 mb-6">
            <div
              v-for="(step, index) in installProgress.steps"
              :key="index"
              class="flex items-start space-x-3"
            >
              <!-- 步骤图标 -->
              <div class="flex-shrink-0 mt-0.5">
                <div
                  v-if="index < installProgress.currentStep"
                  class="w-6 h-6 rounded-full bg-green-500 flex items-center justify-center"
                >
                  <svg class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div
                  v-else-if="index === installProgress.currentStep"
                  class="w-6 h-6 rounded-full bg-blue-500 flex items-center justify-center"
                >
                  <div class="w-3 h-3 bg-white rounded-full animate-pulse"></div>
                </div>
                <div
                  v-else
                  class="w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center"
                >
                  <div class="w-2 h-2 bg-gray-400 rounded-full"></div>
                </div>
              </div>

              <!-- 步骤内容 -->
              <div class="flex-1 min-w-0">
                <p :class="[
                  'text-sm',
                  index < installProgress.currentStep ? 'text-gray-700 font-medium' : 
                  index === installProgress.currentStep ? 'text-blue-600 font-medium' : 
                  'text-gray-500'
                ]">
                  {{ step.name }}
                </p>
                <p v-if="step.message" class="text-xs text-gray-500 mt-0.5">{{ step.message }}</p>
              </div>
            </div>
          </div>

          <!-- 安装结果 -->
          <div v-if="installProgress.currentStep === installProgress.steps.length" class="mb-4 flex-shrink-0">
            <div
              v-if="installProgress.success"
              class="bg-green-50 border border-green-200 rounded-lg p-4"
            >
              <div class="flex items-center space-x-3">
                <div class="flex-shrink-0">
                  <svg class="w-6 h-6 text-green-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-green-800">安装成功</p>
                  <p v-if="installProgress.message" class="text-xs text-green-600 mt-1">{{ installProgress.message }}</p>
                  <p v-if="dryRun" class="text-xs text-yellow-600 mt-1 font-medium">⚠️ 当前为测试模式（dryRun），未执行真实安装</p>
                  <p v-else class="text-xs text-gray-500 mt-1">ℹ️ 真实安装尚未接入，当前仍为模拟</p>
                </div>
              </div>
            </div>
            <div
              v-else
              class="bg-red-50 border border-red-200 rounded-lg p-4"
            >
              <div class="flex items-center space-x-3">
                <div class="flex-shrink-0">
                  <svg class="w-6 h-6 text-red-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-red-800">安装失败</p>
                  <p v-if="installProgress.message" class="text-xs text-red-600 mt-1">{{ installProgress.message }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4 flex-shrink-0">
          <div v-if="installProgress.currentStep < installProgress.steps.length" class="flex items-center justify-center">
            <div class="inline-block animate-spin rounded-full h-6 w-6 border-2 border-gray-200 border-t-blue-600"></div>
            <span class="ml-3 text-sm text-gray-600">正在安装，请稍候...</span>
          </div>
          <div v-else class="flex items-center space-x-3">
            <button
              v-if="installProgress.success"
              @click="printTestPage"
              class="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition-colors flex items-center justify-center space-x-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
              </svg>
              <span>打印测试页</span>
            </button>
            <button
              @click="closeInstallProgress"
              class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
              :disabled="installProgress.currentStep < installProgress.steps.length"
              :class="{
                'opacity-50 cursor-not-allowed': installProgress.currentStep < installProgress.steps.length
              }"
            >
              {{ installProgress.success ? '完成' : '关闭' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 确认对话框 -->
    <div 
      v-if="showConfirmDialog" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="cancelConfirmDialog"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        <!-- 对话框标题 -->
        <div :class="[
          'px-6 py-4 border-b',
          confirmDialog.type === 'danger' ? 'bg-red-50 border-red-200' : 
          confirmDialog.type === 'warning' ? 'bg-yellow-50 border-yellow-200' : 
          'bg-blue-50 border-blue-200'
        ]">
          <div class="flex items-center justify-between">
            <h3 :class="[
              'text-lg font-semibold',
              confirmDialog.type === 'danger' ? 'text-red-900' : 
              confirmDialog.type === 'warning' ? 'text-yellow-900' : 
              'text-blue-900'
            ]">{{ confirmDialog.title }}</h3>
            <button
              @click="cancelConfirmDialog"
              class="text-gray-400 hover:text-gray-600 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6">
          <div class="flex items-start space-x-4">
            <div :class="[
              'rounded-full p-3 flex-shrink-0',
              confirmDialog.type === 'danger' ? 'bg-red-100' : 
              confirmDialog.type === 'warning' ? 'bg-yellow-100' : 
              'bg-blue-100'
            ]">
              <svg :class="[
                'w-6 h-6',
                confirmDialog.type === 'danger' ? 'text-red-600' : 
                confirmDialog.type === 'warning' ? 'text-yellow-600' : 
                'text-blue-600'
              ]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path v-if="confirmDialog.type === 'danger'" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                <path v-else-if="confirmDialog.type === 'warning'" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div class="flex-1">
              <div class="text-sm text-gray-700 whitespace-pre-line">{{ confirmDialog.message }}</div>
            </div>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4">
          <div class="flex items-center space-x-3">
            <button
              @click="cancelConfirmDialog"
              class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
            >
              取消
            </button>
            <button
              @click="confirmDialogAction"
              :class="[
                'flex-1 px-4 py-2 text-sm font-medium text-white rounded-md transition-colors',
                confirmDialog.type === 'danger' ? 'bg-red-600 hover:bg-red-700' : 
                confirmDialog.type === 'warning' ? 'bg-yellow-600 hover:bg-yellow-700' : 
                'bg-blue-600 hover:bg-blue-700'
              ]"
            >
              确定
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误提示对话框 -->
    <div 
      v-if="showErrorDialog" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="closeErrorDialog"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        <!-- 对话框标题 -->
        <div class="bg-red-50 border-b border-red-200 px-6 py-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-red-900">{{ errorDialog.title }}</h3>
            <button
              @click="closeErrorDialog"
              class="text-gray-400 hover:text-gray-600 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6">
          <div class="flex items-start space-x-4">
            <div class="bg-red-100 rounded-full p-3 flex-shrink-0">
              <svg class="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div class="flex-1">
              <div class="text-sm text-gray-700 whitespace-pre-line">{{ errorDialog.message }}</div>
            </div>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4">
          <button
            @click="closeErrorDialog"
            class="w-full px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md transition-colors"
          >
            确定
          </button>
        </div>
      </div>
    </div>

    <!-- 重装进度对话框 -->
    <div 
      v-if="showReinstallProgress" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="handleReinstallProgressBackgroundClick"
    >
      <div 
        class="bg-white rounded-xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden flex flex-col max-h-[90vh]"
        @click.stop
      >
        <!-- 对话框标题 -->
        <div class="bg-gray-50 border-b border-gray-200 px-6 py-4 flex-shrink-0">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">正在重装打印机</h3>
            <button
              v-if="reinstallProgress.currentStep >= reinstallProgress.steps.length"
              @click="closeReinstallProgress"
              class="text-gray-400 hover:text-gray-600 transition-colors"
              title="关闭"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6 flex-1 overflow-y-auto min-h-0">
          <!-- 打印机名称 -->
          <div class="mb-4 flex-shrink-0">
            <h4 class="text-base font-medium text-gray-900 mb-2">{{ reinstallProgress.printerName }}</h4>
            <p v-if="reinstallProgress.printerPath" class="text-xs text-gray-500">{{ reinstallProgress.printerPath }}</p>
          </div>

          <!-- 进度步骤列表 -->
          <div class="space-y-3 mb-6">
            <div
              v-for="(step, index) in reinstallProgress.steps"
              :key="index"
              class="flex items-start space-x-3"
            >
              <!-- 步骤图标 -->
              <div class="flex-shrink-0 mt-0.5">
                <div
                  v-if="index < reinstallProgress.currentStep"
                  class="w-6 h-6 rounded-full bg-green-500 flex items-center justify-center"
                >
                  <svg class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div
                  v-else-if="index === reinstallProgress.currentStep"
                  class="w-6 h-6 rounded-full bg-blue-500 flex items-center justify-center"
                >
                  <div class="w-3 h-3 bg-white rounded-full animate-pulse"></div>
                </div>
                <div
                  v-else
                  class="w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center"
                >
                  <div class="w-2 h-2 bg-gray-400 rounded-full"></div>
                </div>
              </div>

              <!-- 步骤内容 -->
              <div class="flex-1 min-w-0">
                <p :class="[
                  'text-sm',
                  index < reinstallProgress.currentStep ? 'text-gray-700 font-medium' : 
                  index === reinstallProgress.currentStep ? 'text-blue-600 font-medium' : 
                  'text-gray-500'
                ]">
                  {{ step.name }}
                </p>
                <p v-if="step.message" class="text-xs text-gray-500 mt-0.5">{{ step.message }}</p>
              </div>
            </div>
          </div>

          <!-- 重装结果 -->
          <div v-if="reinstallProgress.currentStep === reinstallProgress.steps.length" class="mb-4 flex-shrink-0">
            <div
              v-if="reinstallProgress.success"
              class="bg-green-50 border border-green-200 rounded-lg p-4"
            >
              <div class="flex items-center space-x-3">
                <div class="flex-shrink-0">
                  <svg class="w-6 h-6 text-green-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-green-800">重装成功</p>
                  <p v-if="reinstallProgress.message" class="text-xs text-green-600 mt-1">{{ reinstallProgress.message }}</p>
                </div>
              </div>
            </div>
            <div
              v-else
              class="bg-red-50 border border-red-200 rounded-lg p-4"
            >
              <div class="flex items-center space-x-3">
                <div class="flex-shrink-0">
                  <svg class="w-6 h-6 text-red-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="flex-1">
                  <p class="text-sm font-medium text-red-800">重装失败</p>
                  <p v-if="reinstallProgress.message" class="text-xs text-red-600 mt-1">{{ reinstallProgress.message }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4 flex-shrink-0">
          <div v-if="reinstallProgress.currentStep < reinstallProgress.steps.length" class="flex items-center justify-center">
            <div class="inline-block animate-spin rounded-full h-6 w-6 border-2 border-gray-200 border-t-blue-600"></div>
            <span class="ml-3 text-sm text-gray-600">正在重装，请稍候...</span>
          </div>
          <div v-else class="flex items-center space-x-3">
            <button
              @click="closeReinstallProgress"
              class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
            >
              {{ reinstallProgress.success ? '完成' : '关闭' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 打印测试页结果对话框 -->
    <div 
      v-if="showTestPageResult" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="closeTestPageResult"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        <!-- 对话框标题 -->
        <div :class="[
          'px-6 py-4 flex-shrink-0 border-b',
          testPageResult.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'
        ]">
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-3">
              <div class="flex-shrink-0">
                <svg 
                  v-if="testPageResult.success"
                  class="w-6 h-6 text-green-600" 
                  fill="currentColor" 
                  viewBox="0 0 20 20"
                >
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                </svg>
                <svg 
                  v-else
                  class="w-6 h-6 text-red-600" 
                  fill="currentColor" 
                  viewBox="0 0 20 20"
                >
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                </svg>
              </div>
              <h3 :class="[
                'text-lg font-semibold',
                testPageResult.success ? 'text-green-900' : 'text-red-900'
              ]">
                {{ testPageResult.success ? '打印测试页成功' : '打印测试页失败' }}
              </h3>
            </div>
            <button
              @click="closeTestPageResult"
              :class="[
                'transition-colors',
                testPageResult.success ? 'text-green-400 hover:text-green-600' : 'text-red-400 hover:text-red-600'
              ]"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6">
          <div class="mb-4">
            <p class="text-sm text-gray-700 mb-2">
              <span class="font-medium">打印机:</span> {{ installProgress.printerName }}
            </p>
            <p v-if="testPageResult.message" :class="[
              'text-sm',
              testPageResult.success ? 'text-green-700' : 'text-red-700'
            ]">
              {{ testPageResult.message }}
            </p>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div :class="[
          'px-6 py-4 border-t flex-shrink-0',
          testPageResult.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'
        ]">
          <button
            @click="closeTestPageResult"
            :class="[
              'w-full px-4 py-2 text-sm font-medium rounded-md transition-colors',
              testPageResult.success 
                ? 'text-green-700 bg-white hover:bg-green-100 border border-green-300' 
                : 'text-red-700 bg-white hover:bg-red-100 border border-red-300'
            ]"
          >
            确定
          </button>
        </div>
      </div>
    </div>

    <!-- 版本更新对话框 -->
    <div 
      v-if="showVersionUpdateDialog && versionUpdateInfo" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="closeVersionUpdateDialog"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden flex flex-col max-h-[90vh]">
        <!-- 对话框标题 -->
        <div class="bg-blue-50 border-b border-blue-200 px-6 py-4 flex-shrink-0">
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-3">
              <div class="flex-shrink-0">
                <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
              </div>
              <h3 class="text-lg font-semibold text-blue-900">发现新版本</h3>
            </div>
            <button
              v-if="!versionUpdateInfo.force_update"
              @click="closeVersionUpdateDialog"
              class="text-blue-400 hover:text-blue-600 transition-colors"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- 对话框内容 -->
        <div class="px-6 py-6 flex-1 overflow-y-auto min-h-0">
          <div class="mb-4">
            <p class="text-sm text-gray-700 mb-2">
              <span class="font-medium">当前版本:</span> {{ versionUpdateInfo.current_version }}
            </p>
            <p class="text-sm text-gray-700 mb-2">
              <span class="font-medium">最新版本:</span> 
              <span class="text-blue-600 font-semibold">{{ versionUpdateInfo.latest_version }}</span>
            </p>
            <p v-if="versionUpdateInfo.download_size" class="text-xs text-gray-500 mb-4">
              下载大小: {{ versionUpdateInfo.download_size }}
            </p>
          </div>

          <div v-if="versionUpdateInfo.update_description" class="mb-4">
            <p class="text-sm font-medium text-gray-900 mb-2">更新内容:</p>
            <div class="bg-gray-50 rounded-lg p-4">
              <pre class="text-xs text-gray-700 whitespace-pre-wrap">{{ versionUpdateInfo.update_description }}</pre>
            </div>
          </div>

          <div v-if="versionUpdateInfo.changelog && versionUpdateInfo.changelog.length > 0" class="mb-4">
            <p class="text-sm font-medium text-gray-900 mb-2">更新日志:</p>
            <div class="space-y-3">
              <div 
                v-for="(entry, index) in versionUpdateInfo.changelog.slice(0, 3)" 
                :key="index"
                class="bg-gray-50 rounded-lg p-3"
              >
                <div class="flex items-center justify-between mb-1">
                  <span class="text-sm font-medium text-gray-900">v{{ entry.version }}</span>
                  <span class="text-xs text-gray-500">{{ entry.date }}</span>
                </div>
                <ul class="text-xs text-gray-700 space-y-1">
                  <li v-for="(change, idx) in entry.changes" :key="idx" class="flex items-start">
                    <span class="mr-2">•</span>
                    <span>{{ change }}</span>
                  </li>
                </ul>
              </div>
            </div>
          </div>

          <div v-if="versionUpdateInfo.force_update" class="bg-yellow-50 border border-yellow-200 rounded-lg p-3 mb-4">
            <p class="text-sm text-yellow-800">
              <svg class="w-4 h-4 inline mr-1" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
              </svg>
              此版本包含重要更新，建议立即更新
            </p>
          </div>
        </div>

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4 flex-shrink-0">
          <div class="flex items-center space-x-3">
            <button
              v-if="versionUpdateInfo.update_url"
              @click="downloadAndUpdate"
              class="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition-colors flex items-center justify-center space-x-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
              <span>下载并更新</span>
            </button>
            <button
              v-if="!versionUpdateInfo.force_update"
              @click="closeVersionUpdateDialog"
              class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
            >
              稍后更新
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 调试日志窗口 - 非阻塞式，位于右下角 -->
    <div 
      v-if="showDebugWindow"
      class="fixed bottom-4 right-4 z-[100] w-[600px] max-w-[calc(100vw-2rem)] h-[70vh] max-h-[600px] shadow-2xl rounded-lg overflow-hidden flex flex-col pointer-events-auto"
      style="pointer-events: auto;"
    >
      <div 
        class="bg-white w-full h-full overflow-hidden flex flex-col pointer-events-auto"
      >
        <!-- 调试窗口标题 -->
        <div class="bg-yellow-50 border-b border-yellow-200 px-6 py-4 flex-shrink-0">
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-3">
              <div class="bg-yellow-100 rounded-lg p-2">
                <svg class="w-5 h-5 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
              </div>
              <div>
                <h3 class="text-lg font-semibold text-gray-900">调试日志</h3>
                <p class="text-xs text-gray-500 mt-0.5">共 {{ debugLogs.length }} 条日志</p>
              </div>
            </div>
            <div class="flex items-center space-x-2">
              <button
                @click="clearDebugLogs"
                class="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
                title="清空日志"
              >
                <svg class="w-4 h-4 inline-block mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
                清空
              </button>
              <button
                @click="copyDebugLogs"
                class="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
                title="复制日志"
              >
                <svg class="w-4 h-4 inline-block mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                复制
              </button>
              <button
                @click="closeDebugWindow"
                class="text-gray-400 hover:text-gray-600 transition-colors"
                title="关闭"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <!-- 调试日志内容 -->
        <div class="flex-1 overflow-hidden flex flex-col">
          <!-- 日志类型筛选 -->
          <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center space-x-1 flex-shrink-0">
            <span class="text-xs text-gray-600">筛选:</span>
            <button
              v-for="type in ['all', 'log', 'info', 'warn', 'error']"
              :key="type"
              @click="debugLogFilter = type"
              :class="[
                'px-2 py-0.5 text-xs font-medium rounded transition-colors',
                debugLogFilter === type
                  ? 'bg-yellow-500 text-white'
                  : 'bg-white text-gray-700 hover:bg-gray-100 border border-gray-300'
              ]"
            >
              {{ type === 'all' ? '全部' : type === 'log' ? '日志' : type === 'info' ? '信息' : type === 'warn' ? '警告' : '错误' }}
            </button>
          </div>

          <!-- 日志列表 -->
          <div 
            ref="debugLogContainer"
            class="flex-1 overflow-y-auto px-4 py-2 bg-gray-50 font-mono text-xs"
          >
            <div v-if="filteredDebugLogs.length === 0" class="text-center text-gray-400 py-8">
              暂无日志
            </div>
            <div
              v-for="(log, index) in filteredDebugLogs"
              :key="index"
              :class="[
                'mb-2 p-2 rounded border-l-4',
                log.type === 'error' ? 'bg-red-50 border-red-400 text-red-800' :
                log.type === 'warn' ? 'bg-yellow-50 border-yellow-400 text-yellow-800' :
                log.type === 'info' ? 'bg-blue-50 border-blue-400 text-blue-800' :
                'bg-white border-gray-300 text-gray-700'
              ]"
            >
              <div class="flex items-start space-x-2">
                <span class="text-gray-500 flex-shrink-0">{{ formatLogTime(log.timestamp) }}</span>
                <span :class="[
                  'font-semibold flex-shrink-0 px-1.5 py-0.5 rounded text-xs',
                  log.type === 'error' ? 'bg-red-200 text-red-900' :
                  log.type === 'warn' ? 'bg-yellow-200 text-yellow-900' :
                  log.type === 'info' ? 'bg-blue-200 text-blue-900' :
                  'bg-gray-200 text-gray-900'
                ]">
                  {{ log.type.toUpperCase() }}
                </span>
                <span class="flex-1 break-all">{{ log.message }}</span>
              </div>
              <div v-if="log.stack" class="mt-1 ml-12 text-gray-600 text-xs font-mono">
                {{ log.stack }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
  </div>
  </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/tauri'
import PrinterItem from './components/PrinterItem.vue'
import AppTitleBar from "./components/AppTitleBar.vue"
import { getAppSettings, setAppSettings, getDriverInstallStrategy } from './settings/appSettings'

export default {
  name: 'App',
  components: {
    PrinterItem,
    AppTitleBar
  },
  data() {
    return {
      loading: false,
      error: null,
      config: null,
      installedPrinters: [], // 保留用于兼容，但不再在 loadData 中等待
      selectedAreaIndex: null, // 当前选中的办公区索引
      initialLoadingPrinters: false, // 首次加载打印机状态（显示加载提示）
      // 打印机检测状态管理
      printerDetect: {
        status: 'idle', // 'idle' | 'running' | 'timeout' | 'error'
        error: null
      },
      printerRuntime: {}, // key: printer.name, value: { detectState: 'detecting' | 'installed' | 'not_installed' | 'unknown' }
      statusMessage: '',
      statusType: 'info', // 'info', 'success', 'error'
      dingtalkIcon: '/dingtalk_icon.png', // 钉钉图标路径（从 public 目录）
      showHelp: false, // 显示设置对话框
      settingsTab: 'settings', // 当前选中的tab：'settings' | 'about'
      version: '1.4.1', // 软件版本号
      // 应用设置
      appSettings: {
        remove_port: false,
        remove_driver: false,
        driver_install_strategy: 'always_install_inf' // 默认值：总是安装/更新 INF 驱动（稳定）
      },
      showUpdateDialog: false, // 显示更新对话框
      pendingRemoteConfig: null, // 待更新的远程配置
      localVersion: '', // 本地版本号
      remoteVersion: '', // 远程版本号
      installingPrinters: new Set(), // 正在安装的打印机名称集合（统一管理安装状态）
      reinstallingPrinters: new Set(), // 正在重装的打印机名称集合
      showInstallProgress: false, // 显示安装进度对话框
      installProgress: {
        printerName: '',
        printerPath: '',
        steps: [],
        currentStep: 0,
        success: false,
        message: ''
      },
      showTestPageResult: false, // 显示打印测试页结果对话框
      testPageResult: {
        success: false,
        message: ''
      },
      showReinstallProgress: false, // 显示重装进度对话框
      reinstallProgress: {
        printerName: '',
        printerPath: '',
        steps: [],
        currentStep: 0,
        success: false,
        message: ''
      },
      // 确认对话框状态
      showConfirmDialog: false,
      confirmDialog: {
        title: '',
        message: '',
        type: 'warning', // 'warning' | 'danger' | 'info'
        resolve: null, // Promise resolve 函数
        printer: null // 关联的打印机信息
      },
      // 错误提示对话框状态
      showErrorDialog: false,
      errorDialog: {
        title: '',
        message: ''
      },
      debugMode: true, // 调试模式开关（默认开启，始终记录日志）
      showDebugButton: false, // 调试按钮显示状态（默认隐藏，按 Ctrl+Shift+D 显示）
      showDebugWindow: false, // 显示调试日志窗口
      debugLogs: [], // 调试日志数组
      debugLogFilter: 'all', // 日志筛选：'all', 'log', 'info', 'warn', 'error'
      originalConsole: {}, // 保存原始的 console 方法
      showVersionUpdateDialog: false, // 显示版本更新对话框
      versionUpdateInfo: null, // 版本更新信息
      driverInstallPolicy: 'always', // 驱动安装策略：'always' | 'reuse_if_installed'
      // 安装方式选择器状态（每台打印机独立保存）
      installModeByPrinter: {}, // key: printerKey (name__path), value: InstallMode
      // 测试模式（dryRun）开关，默认开启
      dryRun: true,
      // 作者的其他作品
      otherProducts: [
      {
          name: 'MeowDocs',
          description: '本地优先的 Markdown 笔记与知识管理工具',
          url: 'https://example.com/meowdocs',
          icon: '/MeowDoc.png' // 图标路径（public 目录）
        },
        {
          name: 'Across the Ocean to See You',
          description: '漂洋过海来看你',
          url: 'https://example.com/atotsy',
          icon: '/Across.png' // 图标路径（public 目录）
        }
      ]
    }
  },
  computed: {
    // 当前选中的办公区
    selectedArea() {
      if (this.selectedAreaIndex === null || !this.config || !this.config.areas) {
        return null
      }
      return this.config.areas[this.selectedAreaIndex]
    },
    // 筛选后的调试日志
    filteredDebugLogs() {
      if (this.debugLogFilter === 'all') {
        return this.debugLogs
      }
      return this.debugLogs.filter(log => log.type === this.debugLogFilter)
    }
  },
  async mounted() {
    // 软件一打开就显示加载提示
    this.initialLoadingPrinters = true
    
    // 启动时检查版本更新
    await this.checkVersionUpdate()
    // 然后加载数据
    await this.loadData()
    
    // 确保至少显示2秒
    const minDisplayTime = new Promise(resolve => setTimeout(resolve, 2000))
    const loadStartTime = Date.now()
    
    // 等待打印机检测完成或至少2秒
    await Promise.all([
      this.startDetectInstalledPrinters().catch(() => {}), // 忽略错误，确保 Promise 完成
      minDisplayTime
    ])
    
    // 隐藏加载提示，同时设置 loading 为 false（让办公区和打印机列表同时显示）
    this.initialLoadingPrinters = false
    this.loading = false
    
    this.setupDebugMode()
    // 加载驱动安装策略设置
    this.loadDriverInstallPolicy()
    // 加载应用设置
    this.loadAppSettings()
    // 加载 dryRun 设置
    this.loadDryRunSetting()
    // 设置调试按钮显示功能
    this.setupDebugButtonToggle()
    // 确保根元素可以获得焦点（用于接收键盘事件）
    this.$nextTick(() => {
      if (this.$refs.appFrame) {
        // 不自动聚焦，但确保可以接收键盘事件
        // this.$refs.appFrame.focus()
      }
    })
  },
  beforeUnmount() {
    this.restoreConsole()
  },
  methods: {
    async checkVersionUpdate() {
      try {
        const result = await invoke('check_version_update')
        if (result && result.has_update) {
          // 显示版本更新提示
          this.showVersionUpdateDialog = true
          this.versionUpdateInfo = result
        }
      } catch (err) {
        // 版本检查失败，不影响使用，静默处理
        console.warn('版本检查失败:', err)
      }
    },
    // 打开设置对话框
    openSettings() {
      this.settingsTab = 'settings' // 默认选中"设置"tab
      this.showHelp = true
    },
    // 选择办公区
    selectArea(index) {
      this.selectedAreaIndex = index
      
      // 切换办公区时，初始化当前办公区的安装方式默认值
      if (this.selectedArea && this.selectedArea.printers) {
        this.initInstallModeDefaults(this.selectedArea.printers)
      }
      
      // 切换办公区时，如果打印机检测状态不是 idle，重新检测
      if (this.printerDetect.status !== 'idle') {
        this.startDetectInstalledPrinters()
      }
    },
    // 检查打印机是否已安装（兼容旧逻辑，但优先使用 detectState）
    isInstalled(printerName) {
      // 优先使用新的 detectState
      if (this.printerRuntime[printerName]) {
        return this.printerRuntime[printerName].detectState === 'installed'
      }
      // 降级到旧逻辑（兼容）
      return this.installedPrinters.some(name => 
        name === printerName || 
        name.includes(printerName) ||
        printerName.includes(name)
      )
    },
    // 获取打印机的检测状态
    getPrinterDetectState(printerName) {
      if (this.printerRuntime[printerName]) {
        return this.printerRuntime[printerName].detectState
      }
      return 'unknown'
    },
    // 初始化打印机运行时状态
    initializePrinterRuntime() {
      this.printerRuntime = {}
      if (this.config && this.config.areas) {
        this.config.areas.forEach(area => {
          if (area.printers) {
            area.printers.forEach(printer => {
              // Vue 3 中直接赋值即可，不需要 $set
              this.printerRuntime[printer.name] = {
                detectState: 'detecting'
              }
            })
          }
        })
      }
    },
    // 异步启动检测已安装打印机（带自动重试机制）
    async startDetectInstalledPrinters() {
      // 生成检测任务唯一 ID
      const detectId = `detect_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
      const detectStartTime = performance.now()
      
      // 检查是否已有检测任务在运行
      const isAlreadyRunning = this.printerDetect.status === 'running'
      
      console.log(`[PrinterDetect][Frontend] DETECT_START detect_id=${detectId} timestamp=${detectStartTime.toFixed(2)} status=${this.printerDetect.status} is_running=${isAlreadyRunning} printers_count=${Object.keys(this.printerRuntime).length}`)
      
      // 如果已经在运行，不重复启动
      if (isAlreadyRunning) {
        console.log(`[PrinterDetect][Frontend] DETECT_SKIP detect_id=${detectId} reason=already_running`)
        return
      }
      
      this.printerDetect.status = 'running'
      this.printerDetect.error = null
      
      let attemptCount = 0
      const maxAttempts = 2
      
      for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        attemptCount = attempt
        const attemptStartTime = performance.now()
        const timeoutMs = attempt === 1 ? 8000 : 18000 // 第一次 8s，第二次 18s
        
        console.log(`[PrinterDetect][Frontend] ATTEMPT_START detect_id=${detectId} attempt=${attempt} timeout_ms=${timeoutMs} status=${this.printerDetect.status}`)
        
        try {
          // 调用后端接口（带超时机制）
          console.log(`[PrinterDetect][Frontend] INVOKE_START detect_id=${detectId} attempt=${attempt}`)
          const detectPromise = invoke('list_printers')
          const timeoutPromise = new Promise((resolve) => {
            setTimeout(() => {
              console.log(`[PrinterDetect][Frontend] TIMEOUT_TRIGGERED detect_id=${detectId} attempt=${attempt} timeout_ms=${timeoutMs}`)
              resolve(null) // 超时返回 null
            }, timeoutMs)
          })
          
          const result = await Promise.race([detectPromise, timeoutPromise])
          const attemptElapsed = performance.now() - attemptStartTime
          
          if (result === null) {
            // 超时情况
            console.log(`[PrinterDetect][Frontend] ATTEMPT_TIMEOUT detect_id=${detectId} attempt=${attempt} elapsed_ms=${attemptElapsed.toFixed(2)}`)
            
            // 如果不是最后一次尝试，继续重试（保持 detecting 状态）
            if (attempt < maxAttempts) {
              console.log(`[PrinterDetect][Frontend] AUTO_RETRY detect_id=${detectId} attempt=${attempt} next_attempt=${attempt + 1}`)
              continue // 继续下一次尝试
            } else {
              // 最后一次尝试也超时，标记为失败
              this.printerDetect.status = 'timeout'
              const totalElapsed = performance.now() - detectStartTime
              console.log(`[PrinterDetect][Frontend] DETECT_FINAL_TIMEOUT detect_id=${detectId} total_elapsed_ms=${totalElapsed.toFixed(2)} attempts=${attemptCount} final_state=unknown`)
              
              // 将所有 detecting 状态置为 unknown
              Object.keys(this.printerRuntime).forEach(printerName => {
                if (this.printerRuntime[printerName].detectState === 'detecting') {
                  this.printerRuntime[printerName].detectState = 'unknown'
                }
              })
              return
            }
          } else if (Array.isArray(result)) {
            // 成功返回：更新每个打印机的检测状态
            console.log(`[PrinterDetect][Frontend] INVOKE_RESOLVE detect_id=${detectId} attempt=${attempt} result_length=${result.length} elapsed_ms=${attemptElapsed.toFixed(2)}`)
            
            this.printerDetect.status = 'idle'
            const installedNames = result
            const totalElapsed = performance.now() - detectStartTime
            
            // 更新 installedPrinters（用于兼容）
            this.installedPrinters = installedNames
            
            // 更新每个打印机的 detectState
            let installedCount = 0
            let notInstalledCount = 0
            Object.keys(this.printerRuntime).forEach(printerName => {
              const isInstalled = installedNames.some(name => 
                name === printerName || 
                name.includes(printerName) ||
                printerName.includes(name)
              )
              this.printerRuntime[printerName].detectState = isInstalled ? 'installed' : 'not_installed'
              if (isInstalled) {
                installedCount++
              } else {
                notInstalledCount++
              }
            })
            
            console.log(`[PrinterDetect][Frontend] DETECT_SUCCESS detect_id=${detectId} total_elapsed_ms=${totalElapsed.toFixed(2)} attempts=${attemptCount} installed=${installedCount} not_installed=${notInstalledCount} final_state=installed/not_installed`)
            return // 成功，退出循环
          } else {
            // 异常情况
            throw new Error('返回结果格式异常')
          }
        } catch (err) {
          const attemptElapsed = performance.now() - attemptStartTime
          console.log(`[PrinterDetect][Frontend] INVOKE_REJECT detect_id=${detectId} attempt=${attempt} elapsed_ms=${attemptElapsed.toFixed(2)} error=${err}`)
          console.error(`[PrinterDetect][Frontend] EXCEPTION detect_id=${detectId} attempt=${attempt}`, err)
          if (err && err.stack) {
            console.error(`[PrinterDetect][Frontend] EXCEPTION_STACK detect_id=${detectId}`, err.stack)
          }
          
          // 如果不是最后一次尝试，继续重试（保持 detecting 状态）
          if (attempt < maxAttempts) {
            console.log(`[PrinterDetect][Frontend] AUTO_RETRY detect_id=${detectId} attempt=${attempt} next_attempt=${attempt + 1} reason=exception`)
            continue // 继续下一次尝试
          } else {
            // 最后一次尝试也失败，标记为错误
            console.error('检测已安装打印机失败:', err)
            this.printerDetect.status = 'error'
            this.printerDetect.error = err.toString() || err.message || '未知错误'
            const totalElapsed = performance.now() - detectStartTime
            
            console.log(`[PrinterDetect][Frontend] DETECT_FINAL_ERROR detect_id=${detectId} total_elapsed_ms=${totalElapsed.toFixed(2)} attempts=${attemptCount} final_state=unknown error=${err}`)
            
            // 将所有 detecting 状态置为 unknown
            Object.keys(this.printerRuntime).forEach(printerName => {
              if (this.printerRuntime[printerName].detectState === 'detecting') {
                this.printerRuntime[printerName].detectState = 'unknown'
              }
            })
            return
          }
        }
      }
    },
    // 重试检测
    async retryDetect() {
      // 重置所有 unknown 状态为 detecting
      Object.keys(this.printerRuntime).forEach(printerName => {
        if (this.printerRuntime[printerName].detectState === 'unknown') {
          // Vue 3 中直接赋值即可，不需要 $set
          this.printerRuntime[printerName].detectState = 'detecting'
        }
      })
      
      // 重新启动检测
      await this.startDetectInstalledPrinters()
    },
    async loadData() {
      this.loading = true
      this.error = null
      this.statusMessage = '正在加载配置...'
      this.statusType = 'info'

      try {
        // 只加载配置，不等待打印机列表检测
        const configResult = await invoke('load_config').catch(err => {
          console.error('加载配置失败:', err)
          throw err
        })

        
        // 检查配置结果是否有效
        if (!configResult) {
          throw new Error('配置加载失败：返回结果为空')
        }
        
        if (!configResult.config) {
          throw new Error('配置加载失败：配置数据为空')
        }
        
        if (!configResult.config.areas || configResult.config.areas.length === 0) {
          console.warn('警告：配置中没有打印机区域数据')
          this.statusMessage = '配置加载成功，但未找到打印机数据'
          this.statusType = 'info'
        }

        // 配置加载成功
        this.config = configResult.config
        
        
        // 检查是否有远程更新
        if (configResult.has_remote_update && configResult.remote_config) {
          // 有远程更新，显示更新提示对话框
          this.showUpdateDialog = true
          this.pendingRemoteConfig = configResult.remote_config
          this.localVersion = configResult.local_version || '未知'
          this.remoteVersion = configResult.remote_version || '未知'
          this.statusMessage = '检测到远程配置更新，请确认是否更新'
          this.statusType = 'info'
        } else {
          // 显示配置来源和远程加载状态
          if (configResult.source === 'local') {
            if (configResult.remote_error) {
              // 使用本地配置，但远程加载失败（只提示，不影响使用）
              this.statusMessage = `已加载本地配置（远程更新失败：${configResult.remote_error}）`
              this.statusType = 'info' // 使用 info 而不是 error，因为不影响使用
            } else {
              this.statusMessage = '已加载本地配置'
              this.statusType = 'success'
            }
          } else {
            this.statusMessage = '已加载远程配置'
            this.statusType = 'success'
          }
        }

        // 初始化打印机运行时状态（所有打印机初始为 detecting）
        this.initializePrinterRuntime()
        
        // 初始化安装方式默认值（从配置文件中读取）
        if (this.config && this.config.areas) {
          this.config.areas.forEach(area => {
            if (area.printers) {
              this.initInstallModeDefaults(area.printers)
            }
          })
        }
        
        // 如果有办公区且未选择，自动选择第一个
        if (this.config && this.config.areas && this.config.areas.length > 0 && this.selectedAreaIndex === null) {
          this.selectedAreaIndex = 0
        }
        
        // 如果不是首次加载（initialLoadingPrinters 为 false），立即设置 loading = false
        // 首次加载时，initialLoadingPrinters 为 true，等待 mounted() 中统一控制
        // 这样可以确保首次加载时办公区列表和打印机列表同时显示，刷新时立即显示
        if (!this.initialLoadingPrinters) {
          this.loading = false
        }
      } catch (err) {
        console.error('加载数据时发生错误:', err)
        this.error = err.toString() || err.message || '未知错误'
        this.statusMessage = `加载失败: ${this.error}`
        this.statusType = 'error'
        // 如果加载失败，立即设置 loading = false，显示错误信息
        this.loading = false
      }
    },
    async refresh() {
      await this.loadData()
      // 刷新后重新检测打印机
      this.startDetectInstalledPrinters()
    },
            async handleInstall(printer) {
              // 获取当前选择的安装方式
              const installMode = this.getInstallMode(printer)
              
              // 获取打印机唯一标识 key
              const key = this.getPrinterKey(printer)
              
              // [InstallClick] 打印安装方式（同时输出 mode 和 printer.install_mode 用于对比验证）
              console.log('[InstallClick]', { 
                key,
                name: printer.name,
                mode: installMode,
                configDefault: printer.install_mode || 'auto',
                dryRun: this.dryRun
              })
              
              // 开始安装：添加到 installingPrinters Set
              this.installingPrinters.add(printer.name)
              
              // [UI][InstallClick] 插桩日志 - 记录点击时的状态
              console.log(`[UI][InstallClick] id=${printer.name} before=installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)}`)
              
              console.info('========================================')
              console.info(`🚀 开始安装打印机: ${printer.name}`)
              console.info(`📍 打印机路径: ${printer.path}`)
              console.info(`🔧 驱动路径: ${printer.driver_path || '(未配置)'}`)
              console.info(`📋 型号: ${printer.model || '(未配置)'}`)
              console.info(`🔧 安装方式: ${installMode}`)
              
              if (!printer.driver_path) {
                console.warn('⚠️ 警告: printer.driver_path 为空！可能是配置文件中没有该字段或读取时丢失了')
              }
              
              // 初始化安装进度（按平台显示不同的步骤）
              // macOS: 安全优先，尽量提示用户使用系统方式（lpadmin / 系统设置）或提供 PPD
              // Windows: 保留原有的详细自动化步骤（INF 安装 / Add-Printer 等）
              const isMac = typeof navigator !== 'undefined' && navigator.platform && /Mac|iPhone|iPad|iPod/.test(navigator.platform)
              let steps = []

              if (isMac) {
                // macOS 专用步骤 —— 不主动调用 Windows 特有的安装流程
                if (printer.driver_path) {
                  steps.push({ name: '准备 PPD/驱动 (可选)', message: '' })
                }
                steps.push({ name: '通过系统或 lpadmin 添加打印机', message: '' })
                steps.push({ name: '检查打印机是否在线', message: '' })
              } else {
                // Windows / 其他平台使用原有流程
                steps = [
                  { name: '检查打印机驱动', message: '' },
                  { name: '添加打印机端口', message: '' }
                ]

                if (printer.driver_path) {
                  steps.push({ name: '查找品牌驱动', message: '' })
                  steps.push({ name: '从配置文件安装 INF 驱动', message: '' })
                }

                steps.push(
                  { name: '安装打印机驱动', message: '' },
                  { name: '配置打印机', message: '' },
                  { name: '验证安装', message: '' }
                )
              }
              
              this.installProgress = {
                printerName: printer.name,
                printerPath: printer.path,
                steps: steps,
                currentStep: 0,
                success: false,
                message: ''
              }
              
              // 显示进度对话框
              this.showInstallProgress = true
              this.statusMessage = `正在安装 ${printer.name}...`
              this.statusType = 'info'

              try {
                let stepIndex = 0
                
                // 步骤1: 检查打印机驱动
                console.info(`[步骤 ${stepIndex + 1}] 检查打印机驱动`)
                this.updateProgressStep(stepIndex, '正在检查系统中是否有可用的打印机驱动...')
                await this.delay(300)
                console.info(`[步骤 ${stepIndex + 1}] ✓ 检查完成`)
                stepIndex++
                
                // 步骤2: 添加打印机端口
                console.info(`[步骤 ${stepIndex + 1}] 添加打印机端口`)
                this.updateProgressStep(stepIndex, '正在添加打印机端口...')
                await this.delay(500)
                console.info(`[步骤 ${stepIndex + 1}] ✓ 端口添加完成`)
                stepIndex++
                
                // 如果有配置的驱动路径，添加额外步骤
                if (printer.driver_path) {
                  // 步骤4: 查找品牌驱动
                  console.info(`[步骤 ${stepIndex + 1}] 查找品牌驱动`)
                  this.updateProgressStep(stepIndex, '正在查找品牌驱动...')
                  await this.delay(400)
                  console.info(`[步骤 ${stepIndex + 1}] ✓ 查找完成`)
                  stepIndex++
                  
                  // 步骤5: 从配置文件安装 INF 驱动
                  console.info(`[步骤 ${stepIndex + 1}] 从配置文件安装 INF 驱动: ${printer.driver_path}`)
                  this.updateProgressStep(stepIndex, `正在从配置文件安装 INF 驱动: ${printer.driver_path}...`)
                  await this.delay(600)
                  console.info(`[步骤 ${stepIndex + 1}] ✓ INF 驱动安装完成`)
                  stepIndex++
                }
                
                // 步骤N: 安装打印机驱动
                console.info(`[步骤 ${stepIndex + 1}] 安装打印机驱动`)
                this.updateProgressStep(stepIndex, '正在安装打印机驱动...')
                await this.delay(800)
                console.info(`[步骤 ${stepIndex + 1}] ✓ 驱动安装完成`)
                stepIndex++
                
                // 步骤N+1: 配置打印机
                console.info(`[步骤 ${stepIndex + 1}] 配置打印机`)
                this.updateProgressStep(stepIndex, '正在配置打印机...')
                await this.delay(500)
                
                // 调用后端安装函数（在配置打印机步骤中调用，这样可以实时反映进度）
                // 确保 driver_path 正确传递（处理 undefined、null 和空字符串）
                const driverPathParam = printer.driver_path && printer.driver_path.trim() !== '' 
                  ? printer.driver_path 
                  : null
                const modelParam = printer.model && printer.model.trim() !== '' 
                  ? printer.model 
                  : null
                
                // 尝试使用 camelCase 参数名，因为 Tauri 可能对带下划线的参数名有问题
                const installParams = {
                  name: printer.name,
                  path: printer.path,
                  driverPath: driverPathParam,  // 改为 camelCase，匹配 Rust 端的参数名
                  model: modelParam,
                  // TODO: 后续应使用统一设置接口获取驱动安装策略
                  // 使用方式：const strategy = this.getDriverInstallStrategy()
                  // 然后映射到后端参数：'always_install_inf' -> 'always', 'skip_if_driver_exists' -> 'reuse_if_installed'
                  driverInstallPolicy: this.driverInstallPolicy,  // 驱动安装策略（临时兼容，后续迁移到统一设置）
                  installMode: installMode,  // 安装方式：从 UI 选择器获取（使用 camelCase 匹配 Rust 端）
                  dryRun: this.dryRun  // 测试模式：从设置中获取（使用 camelCase 匹配 Rust 端）
                }
                
                console.info('📤 调用后端安装函数')
                console.info(`参数:`, JSON.stringify(installParams, null, 2))
                console.info(`[Frontend] dryRun=${this.dryRun}, installMode=${installMode}`)
                
                const installPromise = invoke('install_printer', installParams)
                
                // 等待安装完成（不阻塞，但会在后台运行）
                const result = await installPromise
                
                console.info('📥 后端返回结果')
                console.info(`成功: ${result.success}`)
                console.info(`方法: ${result.method || '未知'}`)
                console.info(`消息: ${result.message}`)
                
                // 输出 PowerShell 执行结果到调试模式
                if (result.stdout) {
                  console.log('📋 PowerShell 标准输出:')
                  console.log(result.stdout)
                }
                if (result.stderr) {
                  console.error('❌ PowerShell 错误输出:')
                  console.error(result.stderr)
                }
                
                
                // 步骤N+2: 验证安装
                console.info(`[步骤 ${stepIndex + 1}] 验证安装`)
                this.updateProgressStep(stepIndex, '正在验证安装...')
                await this.delay(300)
                
                if (result.success) {
                  console.info(`[步骤 ${stepIndex + 1}] ✓ 验证通过`)
                  console.info('✅ 打印机安装成功!')
                  
                  // [UI][InstallSuccessSignal] 插桩日志 - 成功信号来源：invoke返回值
                  console.log(`[UI][InstallSuccessSignal] id=${printer.name} message="${result.message || '安装成功'}" source=invoke installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)} state=${this.isInstalled(printer.name) ? 'installed' : 'idle'}`)
                  
                  // 更新步骤为完成
                  if (stepIndex < this.installProgress.steps.length && this.installProgress.steps[stepIndex]) {
                    this.installProgress.steps[stepIndex].message = '验证通过'
                  }
                  
                  // 如果使用了配置文件驱动，更新对应步骤的消息
                  if (printer.driver_path) {
                    // 查找"从配置文件安装 INF 驱动"步骤
                    const infInstallStepIndex = this.installProgress.steps.findIndex(step => 
                      step && step.name === '从配置文件安装 INF 驱动'
                    )
                    if (infInstallStepIndex >= 0 && this.installProgress.steps[infInstallStepIndex]) {
                      this.installProgress.steps[infInstallStepIndex].message = 'INF 驱动安装成功'
                    }
                  }
                  
                  // 显示安装方式和消息
                  const method = result.method || '未知'
                  const dryRunMessage = this.dryRun 
                    ? '（当前为测试模式，未执行真实安装）' 
                    : '（真实安装尚未接入，当前仍为模拟）'
                  this.installProgress.success = true
                  this.installProgress.message = `${result.message || '安装成功'}${dryRunMessage}`
                  this.statusMessage = `${result.message || '安装成功'} [方式: ${method}]${dryRunMessage}`
                  this.statusType = 'success'
                  
                  // 重新检测已安装的打印机列表（异步，不阻塞）
                  this.startDetectInstalledPrinters()
                } else {
                  // 安装失败
                  console.error(`[步骤 ${stepIndex + 1}] ✗ 验证失败`)
                  console.error('❌ 打印机安装失败!')
                  console.error(`错误消息: ${result.message}`)
                  
                  // [UI][InstallSuccessSignal] 插桩日志 - 失败信号来源：invoke返回值
                  console.log(`[UI][InstallSuccessSignal] id=${printer.name} message="${result.message || '安装失败'}" source=invoke installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)} state=${this.isInstalled(printer.name) ? 'installed' : 'idle'}`)
                  
                  if (stepIndex < this.installProgress.steps.length && this.installProgress.steps[stepIndex]) {
                    this.installProgress.steps[stepIndex].message = '验证失败'
                  }
                  
                  // 如果使用了配置文件驱动，更新对应步骤的消息
                  if (printer.driver_path) {
                    // 查找"从配置文件安装 INF 驱动"步骤
                    const infInstallStepIndex = this.installProgress.steps.findIndex(step => 
                      step && step.name === '从配置文件安装 INF 驱动'
                    )
                    if (infInstallStepIndex >= 0 && this.installProgress.steps[infInstallStepIndex]) {
                      this.installProgress.steps[infInstallStepIndex].message = 'INF 驱动安装失败或未找到'
                    }
                  }
                  
                  this.installProgress.success = false
                  const method = result.method || '未知'
                  this.installProgress.message = result.message || '安装失败'
                  this.statusMessage = `${result.message || '安装失败'} [方式: ${method}]`
                  this.statusType = 'error'
                }
                
                // 标记所有步骤完成
                this.installProgress.currentStep = this.installProgress.steps.length
                console.info('========================================')
                console.info('安装过程完成')
                
              } catch (err) {
                console.error('========================================')
                console.error('❌ 安装过程发生异常')
                console.error('异常详情:', err)
                if (err && err.stack) {
                  console.error('调用栈:', err.stack)
                }
                
                // [UI][InstallSuccessSignal] 插桩日志 - 异常情况
                console.log(`[UI][InstallSuccessSignal] id=${printer.name} message="异常: ${err}" source=exception installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)} state=${this.isInstalled(printer.name) ? 'installed' : 'idle'}`)
                
                this.installProgress.success = false
                const errorMessage = err && err.toString ? err.toString() : (typeof err === 'string' ? err : '安装失败')
                this.installProgress.message = errorMessage
                this.statusMessage = `安装失败: ${errorMessage}`
                this.statusType = 'error'
                this.installProgress.currentStep = this.installProgress.steps.length
                console.error('========================================')
              }
              finally {
                // [UI][InstallSuccessSignal] 插桩日志 - finally 块执行
                console.log(`[UI][InstallSuccessSignal] id=${printer.name} message="进入finally块" source=finally installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)} state=${this.isInstalled(printer.name) ? 'installed' : 'idle'}`)
                
                // 关键：无论成功/失败/异常，都要释放按钮状态
                this.installingPrinters.delete(printer.name)
                
                // [UI][InstallSuccessSignal] 插桩日志 - 清理后状态
                console.log(`[UI][InstallSuccessSignal] id=${printer.name} message="已清理installing状态" source=finally installingPrinters.has(${printer.name})=${this.installingPrinters.has(printer.name)} state=${this.isInstalled(printer.name) ? 'installed' : 'idle'}`)

                // 安装进度对话框不再自动关闭，用户需要手动关闭
              }
            },
            updateProgressStep(stepIndex, message) {
              if (stepIndex >= 0 && 
                  stepIndex < this.installProgress.steps.length && 
                  this.installProgress.steps[stepIndex]) {
                this.installProgress.currentStep = stepIndex
                if (message) {
                  this.installProgress.steps[stepIndex].message = message
                }
              } else {
                console.warn(`updateProgressStep: stepIndex ${stepIndex} 超出范围或步骤不存在`)
              }
            },
            delay(ms) {
              return new Promise(resolve => setTimeout(resolve, ms))
            },
            closeInstallProgress() {
              // 只有在安装完成或失败时才允许关闭
              if (this.installProgress.currentStep >= this.installProgress.steps.length) {
                this.showInstallProgress = false
                // 重置进度
                this.installProgress = {
                  printerName: '',
                  printerPath: '',
                  steps: [],
                  currentStep: 0,
                  success: false,
                  message: ''
                }
              }
            },
            handleReinstallProgressBackgroundClick() {
              // 重装进度对话框背景点击处理（仅在完成时允许关闭）
              if (this.reinstallProgress.currentStep >= this.reinstallProgress.steps.length) {
                this.closeReinstallProgress()
              }
            },
            updateReinstallProgressStep(stepIndex, message) {
              if (stepIndex >= 0 && 
                  stepIndex < this.reinstallProgress.steps.length && 
                  this.reinstallProgress.steps[stepIndex]) {
                this.reinstallProgress.currentStep = stepIndex
                if (message) {
                  this.reinstallProgress.steps[stepIndex].message = message
                }
              } else {
                console.warn(`updateReinstallProgressStep: stepIndex ${stepIndex} 超出范围或步骤不存在`)
              }
            },
            closeReinstallProgress() {
              // 只有在重装完成或失败时才允许关闭
              if (this.reinstallProgress.currentStep >= this.reinstallProgress.steps.length) {
                this.showReinstallProgress = false
                // 重置进度
                this.reinstallProgress = {
                  printerName: '',
                  printerPath: '',
                  steps: [],
                  currentStep: 0,
                  success: false,
                  message: ''
                }
              }
            },
            handleInstallProgressBackgroundClick() {
              // 只有在安装完成或失败时才允许通过点击背景关闭
              if (this.installProgress.currentStep >= this.installProgress.steps.length) {
                this.closeInstallProgress()
              }
            },
            async printTestPage() {
              try {
                // 调用后端打印测试页
                const result = await invoke('print_test_page', { 
                  printerName: this.installProgress.printerName
                })
                
                // 显示成功对话框
                this.testPageResult = {
                  success: true,
                  message: result || `测试页已成功发送到打印机: ${this.installProgress.printerName}`
                }
                this.showTestPageResult = true
              } catch (err) {
                console.error('打印测试页失败:', err)
                
                // 显示失败对话框
                this.testPageResult = {
                  success: false,
                  message: err || `打印测试页失败，请确保打印机已连接并可以访问。`
                }
                this.showTestPageResult = true
              }
            },
            // 处理从打印机菜单触发的打印测试页
            async handlePrintTestPage(printer) {
              try {
                console.log(`[PrintTestPage] 开始打印测试页: ${printer.name}`)
                this.statusMessage = `正在打印测试页: ${printer.name}...`
                this.statusType = 'info'
                
                // 调用后端打印测试页
                const result = await invoke('print_test_page', { 
                  printerName: printer.name
                })
                
                console.log(`[PrintTestPage] 打印成功: ${printer.name}`)
                this.statusMessage = `测试页已成功发送到打印机: ${printer.name}`
                this.statusType = 'success'
                
                // 显示成功对话框
                this.testPageResult = {
                  success: true,
                  message: result || `测试页已成功发送到打印机: ${printer.name}`
                }
                this.showTestPageResult = true
              } catch (err) {
                console.error(`[PrintTestPage] 打印失败: ${printer.name}`, err)
                this.statusMessage = `打印测试页失败: ${err.message || err}`
                this.statusType = 'error'
                
                // 显示失败对话框
                this.testPageResult = {
                  success: false,
                  message: err.message || err || `打印测试页失败，请确保打印机已连接并可以访问。`
                }
                this.showTestPageResult = true
              }
            },
            closeTestPageResult() {
              this.showTestPageResult = false
              this.testPageResult = {
                success: false,
                message: ''
              }
            },
            async downloadAndUpdate() {
              if (!this.versionUpdateInfo || !this.versionUpdateInfo.update_url) {
                return
              }
              
              try {
                this.statusMessage = '正在下载更新文件...'
                this.statusType = 'info'
                
                const result = await invoke('download_update', {
                  updateUrl: this.versionUpdateInfo.update_url
                })
                
                this.statusMessage = `更新文件已下载: ${result}。请关闭应用并运行下载的文件进行更新。`
                this.statusType = 'success'
                
                // 关闭对话框
                this.closeVersionUpdateDialog()
                
                // 可选：自动打开下载的文件
                if (this.versionUpdateInfo.update_url) {
                  // 延迟一下，让用户看到提示
                  setTimeout(() => {
                    window.open(this.versionUpdateInfo.update_url, '_blank')
                  }, 1000)
                }
              } catch (err) {
                console.error('下载更新失败:', err)
                this.statusMessage = `下载更新失败: ${err}`
                this.statusType = 'error'
              }
            },
            closeVersionUpdateDialog() {
              this.showVersionUpdateDialog = false
              this.versionUpdateInfo = null
            },
    async openDingTalk() {
      try {
        // 钉钉 URL scheme
        // 格式: dingtalk://dingtalkclient/action/sendmsg?dingtalk_id=钉钉号
        // 
        // 如何获取钉钉号：
        // 1. 打开钉钉应用，点击目标联系人的头像
        // 2. 在个人信息页面下拉，找到"钉钉号"
        // 3. 将钉钉号替换到下面的 URL 中
        
        const dingTalkId = 'plajnt7'
        const dingTalkUrl = `dingtalk://dingtalkclient/action/sendmsg?dingtalk_id=${dingTalkId}`
        
        this.statusMessage = '正在打开钉钉...'
        this.statusType = 'info'
        
        // 使用 Rust 后端命令打开 URL scheme
        await invoke('open_url', { url: dingTalkUrl })
        
        this.statusMessage = '钉钉已打开'
        this.statusType = 'success'
      } catch (err) {
        console.error('打开钉钉失败:', err)
        this.statusMessage = `无法打开钉钉: ${err}。请手动打开钉钉并联系IT热线`
        this.statusType = 'error'
      }
    },
    async openProductUrl(url) {
      try {
        // 使用 Rust 后端命令打开外部链接
        await invoke('open_url', { url })
      } catch (err) {
        console.error('打开链接失败:', err)
        // 如果 invoke 失败，尝试使用 window.open 作为降级方案
        if (typeof window !== 'undefined' && window.open) {
          window.open(url, '_blank')
        }
      }
    },
    async confirmUpdate() {
      // 确认更新，调用后端保存远程配置
      try {
        this.statusMessage = '正在更新配置...'
        this.statusType = 'info'
        this.showUpdateDialog = false
        
        const result = await invoke('confirm_update_config')
        
        if (result && result.config) {
          // 更新成功，重新加载数据
          this.config = result.config
          this.statusMessage = '配置已更新，正在刷新...'
          this.statusType = 'success'
          
          // 重置状态
          this.pendingRemoteConfig = null
          
          // 重新初始化打印机运行时状态并异步检测
          this.initializePrinterRuntime()
          
          // 初始化安装方式默认值（从更新后的配置文件中读取）
          if (this.config && this.config.areas) {
            this.config.areas.forEach(area => {
              if (area.printers) {
                this.initInstallModeDefaults(area.printers)
              }
            })
          }
          
          this.startDetectInstalledPrinters()
          
          // 如果有选中的办公区，保持选中状态
          if (this.selectedAreaIndex !== null && this.config && this.config.areas) {
            // 确保选中的索引仍然有效
            if (this.selectedAreaIndex >= this.config.areas.length) {
              this.selectedAreaIndex = 0
            }
          }
          this.statusMessage = '配置更新成功'
        }
      } catch (err) {
        console.error('更新配置失败:', err)
        this.statusMessage = `更新失败: ${err}`
        this.statusType = 'error'
        // 显示错误时，可以重新显示更新对话框
        this.showUpdateDialog = true
      }
    },
    cancelUpdate() {
      // 取消更新
      this.showUpdateDialog = false
      this.pendingRemoteConfig = null
      this.statusMessage = '已取消更新'
      this.statusType = 'info'
    },
    // 调试模式相关方法
    setupDebugMode() {
      // 保存原始的 console 方法
      this.originalConsole = {
        log: console.log,
        info: console.info,
        warn: console.warn,
        error: console.error
      }
      // 默认启用调试模式（始终记录日志）
      this.enableDebugMode()
    },
    // 全局按键处理（在模板上绑定）
    handleGlobalKeyDown(event) {
      // 检测组合键：Ctrl+Shift+D
      const isCtrlShiftD = (event.ctrlKey || event.metaKey) && 
                           event.shiftKey && 
                           (event.key === 'd' || event.key === 'D' || event.keyCode === 68)
      
      if (isCtrlShiftD) {
        event.preventDefault()
        event.stopPropagation()
        
        if (!this.showDebugButton) {
          this.showDebugButton = true
          console.log('[Debug] 调试按钮已显示（按 Ctrl+Shift+D）')
        }
      }
    },
    // 设置调试按钮显示功能
    setupDebugButtonToggle() {
      // 此方法保留用于未来扩展，当前通过模板上的 @keydown 处理按键
    },
    // 切换调试窗口显示（不修改调试状态）
    toggleDebugWindow() {
      this.showDebugWindow = !this.showDebugWindow
    },
    closeDebugWindow() {
      this.showDebugWindow = false
    },
    // 获取打印机的安装方式
    getInstallMode(printer) {
      const key = this.getPrinterKey(printer)
      return this.installModeByPrinter[key] || 'auto'
    },
    // 设置打印机的安装方式
    setInstallMode(printer, mode) {
      const key = this.getPrinterKey(printer)
      // Vue 3 的响应式系统会自动处理对象属性的添加
      this.installModeByPrinter[key] = mode
    },
    // 获取打印机的唯一标识 key
    getPrinterKey(printer) {
      // 优先使用 printer.id，否则使用 name__path
      if (printer.id) {
        return printer.id
      }
      return `${printer.name}__${printer.path || ''}`
    },
    // 安装方式类型定义和映射
    getInstallModeLabel(mode) {
      const INSTALL_MODE_LABEL = {
        'auto': '自动兼容（推荐）',
        'package': '驱动包安装（推荐）',
        'installer': '厂商安装程序',
        'ipp': '免驱打印（系统通用）',
        'legacy_inf': '传统 INF 安装（老型号）'
      }
      return INSTALL_MODE_LABEL[mode] || '自动兼容（推荐）'
    },
    // 验证安装方式是否有效
    isValidInstallMode(mode) {
      const validModes = ['auto', 'package', 'installer', 'ipp', 'legacy_inf']
      return validModes.includes(mode)
    },
    // 初始化安装方式默认值（从配置文件中读取）
    initInstallModeDefaults(printers) {
      if (!printers || !Array.isArray(printers)) {
        return
      }
      
      printers.forEach(printer => {
        const key = this.getPrinterKey(printer)
        
        // 如果 Map 中已存在该 printerKey 的值，不做任何修改（尊重用户已选）
        if (this.installModeByPrinter[key]) {
          return
        }
        
        // 否则：从 printer.install_mode 读取默认值；若缺失则 "auto"
        let defaultMode = 'auto'
        if (printer.install_mode) {
          // 输入校验：若 printer.install_mode 不是有效枚举，则回退 auto
          if (this.isValidInstallMode(printer.install_mode)) {
            defaultMode = printer.install_mode
          } else {
            console.warn(`[InstallMode] 打印机 "${printer.name}" 的 install_mode="${printer.install_mode}" 无效，回退为 "auto"`)
          }
        }
        
        // 写入 Map
        this.installModeByPrinter[key] = defaultMode
      })
    },
    // 加载驱动安装策略设置
    // 加载应用设置
    loadAppSettings() {
      try {
        const settings = getAppSettings()
        this.appSettings = {
          remove_port: settings.remove_port,
          remove_driver: settings.remove_driver,
          driver_install_strategy: settings.driver_install_strategy
        }
        console.log('[Settings] Loaded app settings:', this.appSettings)
        
        // 兼容旧版本：同步到 driverInstallPolicy（用于调试窗口显示，保持兼容）
        // 注意：这只是为了兼容调试窗口，实际业务逻辑应使用统一设置接口
        this.driverInstallPolicy = settings.driver_install_strategy === 'always_install_inf' 
          ? 'always' 
          : 'reuse_if_installed'
      } catch (error) {
        console.error('[Settings] Failed to load app settings:', error)
        // 使用默认值
        this.appSettings = {
          remove_port: false,
          remove_driver: false,
          driver_install_strategy: 'always_install_inf'
        }
      }
    },
    // 设置项变化时的处理
    // 注意：由于使用了 v-model，值已经自动更新，这里只需要持久化
    onSettingChange(key, value) {
      // 立即持久化（v-model 已经更新了 appSettings）
      setAppSettings({ [key]: value })
      console.log(`[Settings] Setting ${key} changed to ${value}`)
    },
    // 驱动安装策略变化时的处理
    onDriverStrategyChange(value) {
      // 立即持久化（v-model 已经更新了 appSettings）
      setAppSettings({ driver_install_strategy: value })
      console.log(`[Settings] Driver install strategy changed to ${value}`)
      
      // 兼容旧版本：同步到 driverInstallPolicy（用于调试窗口显示，保持兼容）
      this.driverInstallPolicy = value === 'always_install_inf' ? 'always' : 'reuse_if_installed'
    },
    // 获取驱动安装策略（供后续安装/重装功能调用）
    // 这是统一的入口函数，后续安装/重装逻辑应调用此方法获取设置
    getDriverInstallStrategy() {
      return getDriverInstallStrategy()
    },
    // 从路径中提取 IP 地址
    extractIpFromPath(path) {
      if (!path) return null
      // 移除 "\\\\" 或 "\\" 前缀
      let cleanPath = path.replace(/^\\\\+/, '').replace(/^\\+/, '')
      // 提取 IP 地址（IPv4）
      const ipMatch = cleanPath.match(/^(\d+\.\d+\.\d+\.\d+)/)
      return ipMatch ? ipMatch[1] : null
    },
    // 显示确认对话框（返回 Promise）
    showConfirmDialogAsync(title, message, type = 'warning', printer = null) {
      return new Promise((resolve) => {
        this.confirmDialog = {
          title,
          message,
          type,
          resolve,
          printer
        }
        this.showConfirmDialog = true
      })
    },
    // 确认对话框：确定
    confirmDialogAction() {
      if (this.confirmDialog.resolve) {
        this.confirmDialog.resolve(true)
      }
      this.showConfirmDialog = false
      this.confirmDialog = {
        title: '',
        message: '',
        type: 'warning',
        resolve: null,
        printer: null
      }
    },
    // 确认对话框：取消
    cancelConfirmDialog() {
      if (this.confirmDialog.resolve) {
        this.confirmDialog.resolve(false)
      }
      this.showConfirmDialog = false
      this.confirmDialog = {
        title: '',
        message: '',
        type: 'warning',
        resolve: null,
        printer: null
      }
    },
    // 显示错误对话框
    showErrorDialogAsync(title, message) {
      this.errorDialog = {
        title,
        message
      }
      this.showErrorDialog = true
    },
    // 关闭错误对话框
    closeErrorDialog() {
      this.showErrorDialog = false
      this.errorDialog = {
        title: '',
        message: ''
      }
    },
    // 处理重装打印机
    async handleReinstall(printer) {
      // 防止重复调用：如果已经在处理中，直接返回
      if (this.reinstallingPrinters.has(printer.name)) {
        return
      }

      // 提取 IP 地址
      const ip = this.extractIpFromPath(printer.path)
      
      // 二次确认（使用 Vue 对话框）
      const confirmMessage = ip
        ? `将按名称 "${printer.name}" 和 IP "${ip}" 重新安装打印机。\n\n如果系统中已存在同名打印机，将提示您先手动删除后再重试。\n\n可能需要管理员权限。`
        : `将按名称 "${printer.name}" 重新安装打印机。\n\n如果系统中已存在同名打印机，将提示您先手动删除后再重试。\n\n可能需要管理员权限。`
      
      // 使用 Vue 确认对话框
      const confirmed = await this.showConfirmDialogAsync(
        '确认重装（不推荐）',
        confirmMessage,
        'warning',
        printer
      )
      
      if (!confirmed) {
        return
      }

      // 添加到重装集合（确认后才添加，防止重复调用）
      this.reinstallingPrinters.add(printer.name)

      // 初始化重装进度
      const steps = [
        { name: '检查已安装的打印机', message: '' },
        { name: '重新安装打印机', message: '' },
        { name: '验证安装', message: '' }
      ]

      this.reinstallProgress = {
        printerName: printer.name,
        printerPath: printer.path,
        steps: steps,
        currentStep: 0,
        success: false,
        message: ''
      }

      // 显示进度对话框
      this.showReinstallProgress = true
      this.statusMessage = `正在重装 ${printer.name}...`
      this.statusType = 'info'

      try {
        // 读取设置
        const strategy = this.getDriverInstallStrategy()
        
        console.log(`[Reinstall] 开始重装打印机: ${printer.name}, ip=${ip || 'null'}`)
        console.log(`[Reinstall] 设置: strategy=${strategy}`)

        // 步骤 1: 检查已安装的打印机
        this.updateReinstallProgressStep(0, '正在检查已安装的打印机...')
        
        // 调用后端重装（传递 config_name 和 ip）
        // 后端会自动执行：检查 -> 重新安装 -> 写入标签
        const resultPromise = invoke('reinstall_printer', {
          configPrinterKey: printer.name,
          configPrinterPath: printer.path,
          configPrinterName: printer.name,
          driverPath: printer.driver_path || null,
          model: printer.model || null,
          removePort: false, // 不再支持删除端口
          removeDriver: false, // 不再支持删除驱动
          driverInstallStrategy: strategy === 'always_install_inf' ? 'always' : 'reuse_if_installed'
        })

        // 等待一小段时间后更新到安装步骤
        await this.delay(500)
        this.updateReinstallProgressStep(1, '正在重新安装打印机...')

        // 等待后端操作完成
        const result = await resultPromise

        // 步骤 2: 验证安装
        this.updateReinstallProgressStep(2, '正在验证安装...')
        await this.delay(300)

        if (result.success) {
          // 步骤 4: 验证安装
          this.updateReinstallProgressStep(3, '验证安装成功')
          await this.delay(200)

          console.log(`[Reinstall] 重装成功: ${printer.name}`)
          this.statusMessage = `已重装打印机: ${printer.name}`
          this.statusType = 'success'
          
          // 更新进度状态
          this.reinstallProgress.success = true
          this.reinstallProgress.message = result.message || '重装成功'
          this.reinstallProgress.currentStep = this.reinstallProgress.steps.length

          // 自动触发重新检测
          await this.startDetectInstalledPrinters()

          // 2秒后自动关闭进度对话框
          setTimeout(() => {
            if (this.reinstallProgress.currentStep >= this.reinstallProgress.steps.length) {
              this.showReinstallProgress = false
            }
          }, 2000)
        } else {
          throw new Error(result.message || '重装失败')
        }
      } catch (error) {
        console.error(`[Reinstall] 重装失败: ${printer.name}`, error)
        this.statusMessage = `重装失败: ${error.message || error}`
        this.statusType = 'error'
        
        // 更新进度状态
        this.reinstallProgress.success = false
        this.reinstallProgress.message = error.message || error
        this.reinstallProgress.currentStep = this.reinstallProgress.steps.length

        // 使用 Vue 错误对话框显示错误
        this.showErrorDialogAsync(
          '重装失败',
          `${error.message || error}\n\n请以管理员权限运行 ePrinty 或联系 IT`
        )
      } finally {
        // 从重装集合中移除
        this.reinstallingPrinters.delete(printer.name)
      }
    },
    // 加载驱动安装策略设置（已弃用，保留用于兼容）
    // 加载 dryRun 设置
    loadDryRunSetting() {
      try {
        const saved = localStorage.getItem('eprinty_dry_run')
        if (saved !== null) {
          this.dryRun = saved === 'true'
        } else {
          // 默认值：true（安全策略）
          this.dryRun = true
          localStorage.setItem('eprinty_dry_run', 'true')
        }
      } catch (err) {
        console.error('加载 dryRun 设置失败:', err)
        this.dryRun = true // 默认值
      }
    },
    // dryRun 设置变更
    onDryRunChange() {
      try {
        localStorage.setItem('eprinty_dry_run', this.dryRun.toString())
        console.log('[Settings] dryRun 已更新:', this.dryRun)
      } catch (err) {
        console.error('保存 dryRun 设置失败:', err)
      }
    },
    // 注意：此方法已迁移到统一设置系统，实际设置应从 getAppSettings() 读取
    // 此方法仅用于调试窗口显示兼容
    loadDriverInstallPolicy() {
      // 不再从独立的 localStorage key 读取，而是从统一设置读取
      // 此方法在 loadAppSettings() 中已被调用，这里只做兼容处理
      try {
        // 如果统一设置已加载，driverInstallPolicy 应该已经同步
        // 如果没有，则使用默认值
        if (!this.driverInstallPolicy) {
          this.driverInstallPolicy = 'always'
        }
      } catch (err) {
        console.warn('加载驱动安装策略设置失败:', err)
        this.driverInstallPolicy = 'always'
      }
    },
    // 保存驱动安装策略设置（已弃用，保留用于兼容）
    // 注意：此方法已迁移到统一设置系统，实际设置应通过 setAppSettings() 保存
    // 此方法仅用于调试窗口显示兼容，实际不会保存到独立 key
    saveDriverInstallPolicy() {
      // 不再保存到独立的 localStorage key，而是同步到统一设置
      try {
        // 将旧值映射到新值并保存到统一设置
        const newValue = this.driverInstallPolicy === 'always' 
          ? 'always_install_inf' 
          : 'skip_if_driver_exists'
        setAppSettings({ driver_install_strategy: newValue })
        console.info(`驱动安装策略已保存到统一设置: ${newValue}`)
      } catch (err) {
        console.error('保存驱动安装策略设置失败:', err)
      }
    },
    enableDebugMode() {
      // 拦截 console 方法
      const self = this
      console.log = function(...args) {
        self.addDebugLog('log', args.join(' '))
        self.originalConsole.log.apply(console, args)
      }
      console.info = function(...args) {
        self.addDebugLog('info', args.join(' '))
        self.originalConsole.info.apply(console, args)
      }
      console.warn = function(...args) {
        self.addDebugLog('warn', args.join(' '))
        self.originalConsole.warn.apply(console, args)
      }
      console.error = function(...args) {
        const error = args[0] instanceof Error ? args[0] : null
        const message = error ? error.message : args.join(' ')
        const stack = error ? error.stack : null
        self.addDebugLog('error', message, stack)
        self.originalConsole.error.apply(console, args)
      }
      
      // 拦截未捕获的错误
      window.addEventListener('error', (event) => {
        this.addDebugLog('error', `Uncaught Error: ${event.message}`, event.error?.stack)
      }, { once: false })
      
      // 拦截未处理的 Promise 拒绝
      window.addEventListener('unhandledrejection', (event) => {
        this.addDebugLog('error', `Unhandled Promise Rejection: ${event.reason}`, event.reason?.stack)
      }, { once: false })
      
      this.addDebugLog('info', '调试模式已启用')
    },
    disableDebugMode() {
      // 恢复原始的 console 方法
      if (this.originalConsole.log) {
        console.log = this.originalConsole.log
        console.info = this.originalConsole.info
        console.warn = this.originalConsole.warn
        console.error = this.originalConsole.error
      }
      this.addDebugLog('info', '调试模式已禁用')
    },
    restoreConsole() {
      // 组件销毁时恢复 console（调试模式始终开启，但组件销毁时需要恢复原始 console）
      if (this.originalConsole.log) {
        console.log = this.originalConsole.log
        console.info = this.originalConsole.info
        console.warn = this.originalConsole.warn
        console.error = this.originalConsole.error
      }
    },
    addDebugLog(type, message, stack = null) {
      // 调试模式始终开启，始终记录日志
      
      const timestamp = new Date()
      this.debugLogs.push({
        type,
        message,
        stack,
        timestamp
      })
      
      // 限制日志数量（最多保留 1000 条）
      if (this.debugLogs.length > 1000) {
        this.debugLogs.shift()
      }
      
      // 自动滚动到底部
      this.$nextTick(() => {
        this.scrollDebugLogsToBottom()
      })
    },
    scrollDebugLogsToBottom() {
      if (this.$refs.debugLogContainer) {
        const container = this.$refs.debugLogContainer
        container.scrollTop = container.scrollHeight
      }
    },
    clearDebugLogs() {
      this.debugLogs = []
    },
    async copyDebugLogs() {
      const logsText = this.filteredDebugLogs.map(log => {
        const time = this.formatLogTime(log.timestamp)
        const type = log.type.toUpperCase()
        let text = `[${time}] ${type}: ${log.message}`
        if (log.stack) {
          text += `\n${log.stack}`
        }
        return text
      }).join('\n\n')
      
      try {
        await navigator.clipboard.writeText(logsText)
        this.addDebugLog('info', '日志已复制到剪贴板')
        alert('日志已复制到剪贴板')
      } catch (err) {
        this.addDebugLog('error', `复制日志失败: ${err.message}`)
        alert('复制失败，请手动选择文本复制')
      }
    },
    formatLogTime(timestamp) {
      const date = new Date(timestamp)
      const hours = String(date.getHours()).padStart(2, '0')
      const minutes = String(date.getMinutes()).padStart(2, '0')
      const seconds = String(date.getSeconds()).padStart(2, '0')
      const milliseconds = String(date.getMilliseconds()).padStart(3, '0')
      return `${hours}:${minutes}:${seconds}.${milliseconds}`
    }
  }
}
</script>


