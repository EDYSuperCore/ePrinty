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

    <!-- 安装按钮 -->
    <div class="flex-shrink-0 ml-4">
      <button
        v-if="!isInstalled"
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
      <div v-else class="px-3 py-1.5 text-xs font-medium text-gray-600">
        已就绪
      </div>
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
    }
  },
  data() {
    return {
      installing: false
    }
  },
  watch: {
    // 当安装状态改变时，重置安装中状态
    isInstalled() {
      this.installing = false
    }
  },
  methods: {
    handleInstall() {
      console.log('PrinterItem: 点击安装按钮', this.printer)
      this.installing = true

      // 传一个 done 回调给父组件，父组件 finally 里调用它
      const done = () => {
        this.installing = false
      }

      this.$emit('install', this.printer, done)
    }
  }
}
</script>

