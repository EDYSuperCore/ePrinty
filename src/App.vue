<template>
  <div class="flex flex-col h-screen bg-gray-50">
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
      version: '1.0.0' // 软件版本号
    }
  },
  computed: {
    // 当前选中的办公区
    selectedArea() {
      if (this.selectedAreaIndex === null || !this.config || !this.config.areas) {
        return null
      }
      return this.config.areas[this.selectedAreaIndex]
    }
  },
  mounted() {
    this.loadData()
  },
  methods: {
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
        console.log('开始加载配置...')
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

        console.log('配置加载结果:', configResult)
        
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

        this.installedPrinters = printers || []
        console.log('已安装的打印机:', this.installedPrinters)
        console.log('配置的打印机区域数:', this.config?.areas?.length || 0)
        
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
              console.log('开始安装打印机:', printer)
              this.statusMessage = `正在安装 ${printer.name}...`
              this.statusType = 'info'

              try {
                // 传递打印机名称和路径
                console.log('调用 install_printer:', { name: printer.name, path: printer.path })
                const result = await invoke('install_printer', { 
                  name: printer.name,
                  path: printer.path 
                })
                
                console.log('安装结果:', result)
                
                if (result.success) {
                  // 显示安装方式和消息
                  const method = result.method || '未知'
                  this.statusMessage = `${result.message} [方式: ${method}]`
                  this.statusType = 'success'
                  // 重新获取已安装的打印机列表
                  try {
                    this.installedPrinters = await invoke('list_printers')
                    console.log('已更新打印机列表:', this.installedPrinters)
                  } catch (e) {
                    console.error('获取打印机列表失败:', e)
                  }
                } else {
                  // 显示安装方式和错误消息
                  const method = result.method || '未知'
                  this.statusMessage = `${result.message} [方式: ${method}]`
                  this.statusType = 'error'
                }
              } catch (err) {
                console.error('安装失败:', err)
                this.statusMessage = `安装失败: ${err}`
                this.statusType = 'error'
              }
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
        
        console.log('打开钉钉聊天:', dingTalkUrl)
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
    }
  }
}
</script>

