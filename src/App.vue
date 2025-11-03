<template>
  <div class="flex flex-col h-screen bg-gray-50" @contextmenu.prevent>
    <!-- 顶部标题 -->
    <header class="bg-white border-b border-gray-200 px-6 py-4 backdrop-blur-xl bg-white/80">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-3">
          <div class="bg-gray-100 rounded-lg p-2">
            <svg class="w-5 h-5 text-gray-700" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
            </svg>
          </div>
          <div>
            <h1 class="text-xl font-semibold text-gray-900">易点云打印机安装小精灵</h1>
            <p class="text-xs text-gray-500 mt-0.5">企业内网打印机管理工具</p>
          </div>
        </div>
        <div class="flex items-center space-x-2">
          <!-- 调试模式按钮 -->
          <button
            @click="toggleDebugMode"
            :class="[
              'flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-all duration-200',
              debugMode 
                ? 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200' 
                : 'text-gray-700 hover:bg-gray-100'
            ]"
            :title="debugMode ? '关闭调试模式' : '开启调试模式'"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <span>调试</span>
            <span v-if="debugLogs.length > 0" class="ml-1 px-1.5 py-0.5 text-xs bg-red-500 text-white rounded-full">
              {{ debugLogs.length }}
            </span>
          </button>
          <!-- 帮助按钮 -->
          <button
            @click="showHelp = true"
            class="flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-lg transition-all duration-200"
            title="帮助"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>帮助</span>
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
      </div>
    </header>

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
        <!-- 加载状态 -->
        <div v-if="loading" class="flex items-center justify-center h-full">
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
                @install="handleInstall"
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

    <!-- 帮助信息对话框 -->
    <div 
      v-if="showHelp" 
      class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
      @click.self="showHelp = false"
    >
      <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        <!-- 对话框标题 -->
        <div class="bg-gray-50 border-b border-gray-200 px-6 py-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">关于</h3>
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

        <!-- 对话框内容 -->
        <div class="px-6 py-6">
          <div class="flex items-center space-x-4 mb-6">
            <div class="bg-gray-100 rounded-xl p-4">
              <svg class="w-10 h-10 text-gray-700" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
              </svg>
            </div>
            <div>
              <h4 class="text-xl font-semibold text-gray-900">易点云打印机安装小精灵</h4>
              <p class="text-sm text-gray-500 mt-1">企业内网打印机管理工具</p>
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

        <!-- 对话框底部 -->
        <div class="bg-gray-50 border-t border-gray-200 px-6 py-4">
          <button
            @click="showHelp = false"
            class="w-full px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
          >
            关闭
          </button>
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
</template>

<script>
import { invoke } from '@tauri-apps/api/tauri'
import PrinterItem from './components/PrinterItem.vue'

export default {
  name: 'App',
  components: {
    PrinterItem
  },
  data() {
    return {
      loading: false,
      error: null,
      config: null,
      installedPrinters: [],
      selectedAreaIndex: null, // 当前选中的办公区索引
      statusMessage: '',
      statusType: 'info', // 'info', 'success', 'error'
      dingtalkIcon: '/dingtalk_icon.png', // 钉钉图标路径（从 public 目录）
      showHelp: false, // 显示帮助对话框
      version: '1.2.0', // 软件版本号
      showUpdateDialog: false, // 显示更新对话框
      pendingRemoteConfig: null, // 待更新的远程配置
      localVersion: '', // 本地版本号
      remoteVersion: '', // 远程版本号
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
      debugMode: false, // 调试模式开关
      showDebugWindow: false, // 显示调试日志窗口
      debugLogs: [], // 调试日志数组
      debugLogFilter: 'all', // 日志筛选：'all', 'log', 'info', 'warn', 'error'
      originalConsole: {}, // 保存原始的 console 方法
      showVersionUpdateDialog: false, // 显示版本更新对话框
      versionUpdateInfo: null // 版本更新信息
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
    // 启动时检查版本更新
    await this.checkVersionUpdate()
    // 然后加载数据
    this.loadData()
    this.setupDebugMode()
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
    // 选择办公区
    selectArea(index) {
      this.selectedAreaIndex = index
    },
    // 检查打印机是否已安装
    isInstalled(printerName) {
      return this.installedPrinters.some(name => 
        name === printerName || 
        name.includes(printerName) ||
        printerName.includes(name)
      )
    },
    async loadData() {
      this.loading = true
      this.error = null
      this.statusMessage = '正在加载配置...'
      this.statusType = 'info'

      try {
        // 并行加载配置和已安装打印机列表
        const [configResult, printers] = await Promise.all([
          invoke('load_config').catch(err => {
            console.error('加载配置失败:', err)
            throw err
          }),
          invoke('list_printers').catch(err => {
            console.warn('获取打印机列表失败:', err)
            return [] // 失败时返回空数组
          })
        ])

        
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

        this.installedPrinters = printers || []
        
        // 如果有办公区且未选择，自动选择第一个
        if (this.config && this.config.areas && this.config.areas.length > 0 && this.selectedAreaIndex === null) {
          this.selectedAreaIndex = 0
        }
      } catch (err) {
        console.error('加载数据时发生错误:', err)
        this.error = err.toString() || err.message || '未知错误'
        this.statusMessage = `加载失败: ${this.error}`
        this.statusType = 'error'
      } finally {
        this.loading = false
      }
    },
    async refresh() {
      await this.loadData()
    },
            async handleInstall(printer) {
              console.info('========================================')
              console.info(`🚀 开始安装打印机: ${printer.name}`)
              console.info(`📍 打印机路径: ${printer.path}`)
              console.info(`🔧 驱动路径: ${printer.driver_path || '(未配置)'}`)
              console.info(`📋 型号: ${printer.model || '(未配置)'}`)
              
              if (!printer.driver_path) {
                console.warn('⚠️ 警告: printer.driver_path 为空！可能是配置文件中没有该字段或读取时丢失了')
              }
              
              // 初始化安装进度
              // 根据是否有配置的驱动路径，动态调整步骤
              const steps = [
                { name: '检查打印机驱动', message: '' },
                { name: '删除旧打印机（如存在）', message: '' },
                { name: '添加打印机端口', message: '' }
              ]
              
              // 如果有配置的驱动路径，添加"从配置文件安装 INF 驱动"步骤
              if (printer.driver_path) {
                steps.push({ name: '查找品牌驱动', message: '' })
                steps.push({ name: '从配置文件安装 INF 驱动', message: '' })
              }
              
              steps.push(
                { name: '安装打印机驱动', message: '' },
                { name: '配置打印机', message: '' },
                { name: '验证安装', message: '' }
              )
              
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
                
                // 步骤2: 删除旧打印机
                console.info(`[步骤 ${stepIndex + 1}] 删除旧打印机（如果存在）`)
                this.updateProgressStep(stepIndex, '正在删除旧打印机（如果存在）...')
                await this.delay(200)
                console.info(`[步骤 ${stepIndex + 1}] ✓ 删除完成`)
                stepIndex++
                
                // 步骤3: 添加打印机端口
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
                  model: modelParam
                }
                
                console.info('📤 调用后端安装函数')
                console.info(`参数:`, JSON.stringify(installParams, null, 2))
                
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
                  this.installProgress.success = true
                  this.installProgress.message = result.message || '安装成功'
                  this.statusMessage = `${result.message || '安装成功'} [方式: ${method}]`
                  this.statusType = 'success'
                  
                  // 重新获取已安装的打印机列表
                  try {
                    this.installedPrinters = await invoke('list_printers')
                  } catch (e) {
                    console.error('获取打印机列表失败:', e)
                  }
                } else {
                  // 安装失败
                  console.error(`[步骤 ${stepIndex + 1}] ✗ 验证失败`)
                  console.error('❌ 打印机安装失败!')
                  console.error(`错误消息: ${result.message}`)
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
                this.installProgress.success = false
                const errorMessage = err && err.toString ? err.toString() : (typeof err === 'string' ? err : '安装失败')
                this.installProgress.message = errorMessage
                this.statusMessage = `安装失败: ${errorMessage}`
                this.statusType = 'error'
                this.installProgress.currentStep = this.installProgress.steps.length
                console.error('========================================')
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
          
          // 重新加载已安装打印机列表
          try {
            this.installedPrinters = await invoke('list_printers')
            // 如果有选中的办公区，保持选中状态
            if (this.selectedAreaIndex !== null && this.config && this.config.areas) {
              // 确保选中的索引仍然有效
              if (this.selectedAreaIndex >= this.config.areas.length) {
                this.selectedAreaIndex = 0
              }
            }
            this.statusMessage = '配置更新成功'
          } catch (e) {
            console.error('获取打印机列表失败:', e)
          }
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
    },
    toggleDebugMode() {
      // 如果窗口已打开，只是关闭窗口
      if (this.showDebugWindow) {
        this.showDebugWindow = false
        return
      }
      
      // 如果窗口未打开，切换调试模式
      this.debugMode = !this.debugMode
      if (this.debugMode) {
        this.enableDebugMode()
        this.showDebugWindow = true
      } else {
        this.disableDebugMode()
        this.showDebugWindow = false
      }
    },
    closeDebugWindow() {
      this.showDebugWindow = false
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
      // 组件销毁时恢复 console
      if (this.debugMode) {
        this.disableDebugMode()
      }
    },
    addDebugLog(type, message, stack = null) {
      if (!this.debugMode) return
      
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

