<template>
  <!-- 删除打印机确认对话框（Windows 清理级别选择） -->
  <div 
    v-if="show && !isMacOS"
    class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50 backdrop-blur-sm"
    @click.self="onCancel"
  >
    <div class="bg-white rounded-xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden">
      <!-- 对话框标题 -->
      <div class="px-6 py-4 border-b bg-yellow-50 border-yellow-200">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-yellow-900">确认删除打印机？</h3>
          <button
            @click="onCancel"
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
        <div class="mb-4">
          <p class="text-sm text-gray-700 mb-4">
            你可以选择删除范围。范围越大，清理越彻底，但可能影响其他打印机。
          </p>
          
          <!-- 单选选项 -->
          <div class="space-y-3">
            <!-- 选项 A: 仅删除队列 -->
            <label class="flex items-start space-x-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors"
              :class="selectedLevel === 'queue' ? 'border-blue-500 bg-blue-50' : 'border-gray-200'">
              <input
                type="radio"
                value="queue"
                v-model="selectedLevel"
                class="mt-1 h-4 w-4 text-blue-600 focus:ring-blue-500"
              />
              <div class="flex-1">
                <div class="font-medium text-gray-900">仅删除打印机（推荐）</div>
                <div class="text-xs text-gray-500 mt-1">移除该打印机队列，不影响驱动和其他打印机</div>
              </div>
            </label>

            <!-- 选项 B: 删除队列 + 端口 -->
            <label class="flex items-start space-x-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors"
              :class="selectedLevel === 'queue_port' ? 'border-blue-500 bg-blue-50' : 'border-gray-200'">
              <input
                type="radio"
                value="queue_port"
                v-model="selectedLevel"
                class="mt-1 h-4 w-4 text-blue-600 focus:ring-blue-500"
              />
              <div class="flex-1">
                <div class="font-medium text-gray-900">删除打印机 + 端口（网络异常时推荐）</div>
                <div class="text-xs text-gray-500 mt-1">同时删除网络端口，下次安装会重新创建</div>
              </div>
            </label>

            <!-- 选项 C: 彻底清理 -->
            <label class="flex items-start space-x-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors"
              :class="selectedLevel === 'full' ? 'border-red-500 bg-red-50' : 'border-gray-200'">
              <input
                type="radio"
                value="full"
                v-model="selectedLevel"
                class="mt-1 h-4 w-4 text-red-600 focus:ring-red-500"
              />
              <div class="flex-1">
                <div class="font-medium text-gray-900">彻底清理（高级）</div>
                <div class="text-xs text-gray-500 mt-1">同时删除驱动。可能影响使用相同驱动的其他打印机</div>
              </div>
            </label>
          </div>
        </div>
      </div>

      <!-- 对话框底部 -->
      <div class="bg-gray-50 border-t border-gray-200 px-6 py-4">
        <div class="flex items-center space-x-3">
          <button
            @click="onCancel"
            class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-white hover:bg-gray-100 border border-gray-300 rounded-md transition-colors"
          >
            取消
          </button>
          <button
            @click="onConfirm"
            class="flex-1 px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md transition-colors"
          >
            删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface Props {
  show: boolean
  printer: any
  isMacOS: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  confirm: [level: string]
  cancel: []
}>()

const selectedLevel = ref<string>('queue')

const onConfirm = () => {
  emit('confirm', selectedLevel.value)
}

const onCancel = () => {
  emit('cancel')
}
</script>
