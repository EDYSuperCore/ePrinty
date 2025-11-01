<template>
  <div class="flex flex-col h-screen bg-gray-50">
    <!-- 顶部标题 -->
    <header class="bg-white shadow-sm border-b border-gray-200 px-6 py-4">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold text-gray-800">易点云打印机安装小精灵</h1>
        <!-- 帮助按钮：IT热线 -->
        <button
          @click="openDingTalk"
          class="flex items-center space-x-2 px-3 py-2 text-sm text-gray-700 hover:text-blue-600 hover:bg-blue-50 rounded-md transition-colors"
          title="IT热线"
        >
          <!-- 钉钉图标 -->
          <img :src="dingtalkIcon" alt="钉钉" class="w-5 h-5 object-contain" />
          <span class="font-medium">IT热线</span>
        </button>
      </div>
    </header>

    <!-- 主体内容 -->
    <div class="flex-1 flex overflow-hidden">
      <!-- 左侧：办公区选择器 -->
      <aside class="w-64 bg-white border-r border-gray-200 flex flex-col">
        <div class="p-4 border-b border-gray-200">
          <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">选择办公区</h2>
        </div>
        
        <!-- 加载状态 -->
        <div v-if="loading" class="flex-1 flex items-center justify-center p-4">
          <div class="text-center">
            <div class="inline-block animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500 mb-2"></div>
            <p class="text-sm text-gray-500">加载中...</p>
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
              'w-full px-4 py-3 text-left border-b border-gray-100 transition-colors',
              selectedAreaIndex === index 
                ? 'bg-blue-50 text-blue-700 border-l-4 border-l-blue-500' 
                : 'hover:bg-gray-50 text-gray-700'
            ]"
          >
            <div class="flex items-center justify-between">
              <span class="font-medium">{{ area.name }}</span>
              <span class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded-full">
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
            <div class="inline-block animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500 mb-4"></div>
            <p class="text-gray-600">正在加载打印机配置...</p>
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-else-if="error" class="flex items-center justify-center h-full">
          <div class="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md">
            <div class="flex items-center mb-2">
              <svg class="w-6 h-6 text-red-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <h3 class="text-lg font-semibold text-red-800">加载失败</h3>
            </div>
            <p class="text-red-600 mb-4">{{ error }}</p>
            <button 
              @click="loadData"
              class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
            >
              重试
            </button>
          </div>
        </div>

        <!-- 未选择办公区提示 -->
        <div v-else-if="selectedAreaIndex === null" class="flex items-center justify-center h-full">
          <div class="text-center">
            <svg class="w-16 h-16 text-gray-300 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
            </svg>
            <p class="text-gray-500 text-lg mb-2">请先选择办公区</p>
            <p class="text-gray-400 text-sm">从左侧列表中选择一个办公区查看打印机</p>
          </div>
        </div>

        <!-- 选中的办公区打印机列表 -->
        <div v-else-if="selectedArea" class="space-y-4">
          <div class="bg-white rounded-lg shadow-md border border-gray-200 overflow-hidden">
            <!-- 办公区标题 -->
            <div class="bg-gradient-to-r from-blue-500 to-blue-600 px-6 py-4">
              <div class="flex items-center justify-between">
                <h2 class="text-lg font-semibold text-white">{{ selectedArea.name }}</h2>
                <span class="text-sm text-blue-100 bg-blue-400 bg-opacity-30 px-3 py-1 rounded-full">
                  {{ selectedArea.printers ? selectedArea.printers.length : 0 }} 台打印机
                </span>
              </div>
            </div>

            <!-- 打印机列表 -->
            <div class="p-4 space-y-3">
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
    <footer class="bg-white border-t border-gray-200 px-6 py-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-2">
          <span class="text-sm text-gray-600">状态:</span>
          <span :class="[
            'text-sm font-medium',
            statusType === 'success' ? 'text-green-600' : 
            statusType === 'error' ? 'text-red-600' : 
            'text-gray-600'
          ]">
            {{ statusMessage || '就绪' }}
          </span>
        </div>
        <button
          @click="refresh"
          class="px-3 py-1.5 text-sm bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
        >
          刷新
        </button>
      </div>
    </footer>
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
      dingtalkIcon: '/dingtalk_icon.png' // 钉钉图标路径（从 public 目录）
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

