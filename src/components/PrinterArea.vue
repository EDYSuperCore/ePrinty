<template>
  <div class="bg-white rounded-lg shadow-md border border-gray-200 overflow-hidden">
    <!-- 区域标题 -->
    <div class="bg-gradient-to-r from-blue-500 to-blue-600 px-6 py-3">
      <h2 class="text-lg font-semibold text-white">{{ area.name }}</h2>
    </div>

    <!-- 打印机列表 -->
    <div class="p-4 space-y-3">
      <PrinterItem
        v-for="printer in area.printers"
        :key="printer.name"
        :printer="printer"
        :is-installed="isInstalled(printer.name)"
        @install="$emit('install', printer)"
      />
    </div>
  </div>
</template>

<script>
import PrinterItem from './PrinterItem.vue'

export default {
  name: 'PrinterArea',
  components: {
    PrinterItem
  },
  props: {
    area: {
      type: Object,
      required: true
    },
    installedPrinters: {
      type: Array,
      default: () => []
    }
  },
  methods: {
    isInstalled(printerName) {
      return this.installedPrinters.some(name => 
        name === printerName || 
        name.includes(printerName) ||
        printerName.includes(name)
      )
    }
  }
}
</script>

