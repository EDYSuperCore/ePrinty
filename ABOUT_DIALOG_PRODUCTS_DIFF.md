# 关于弹窗增加"作者的其他作品"区块 - 代码 Diff

## 修改文件
- `src/App.vue`

## 修改内容

### 1. 数据定义（data() 部分）

**位置**：`src/App.vue` 第 920-927 行

```diff
      showVersionUpdateDialog: false, // 显示版本更新对话框
      versionUpdateInfo: null, // 版本更新信息
+      // 作者的其他作品
+      otherProducts: [
+        {
+          name: 'MeowDocs',
+          description: '本地优先的 Markdown 笔记与知识管理工具',
+          url: 'https://example.com/meowdocs'
+        }
+      ]
    }
  },
```

### 2. 模板部分（在作者信息下方）

**位置**：`src/App.vue` 第 299-319 行

```diff
            </div>
          </div>

+          <!-- 作者的其他作品 -->
+          <div v-if="otherProducts && otherProducts.length > 0" class="mt-6 pt-6 border-t border-gray-200">
+            <p class="text-xs text-gray-500 mb-3">作者的其他作品</p>
+            <div class="space-y-3">
+              <div
+                v-for="product in otherProducts"
+                :key="product.name"
+                class="flex items-start justify-between"
+              >
+                <div class="flex-1 min-w-0">
+                  <p class="text-sm font-medium text-gray-900 mb-0.5">{{ product.name }}</p>
+                  <p class="text-xs text-gray-500">{{ product.description }}</p>
+                </div>
+                <button
+                  @click="openProductUrl(product.url)"
+                  class="ml-3 text-xs text-gray-600 hover:text-gray-900 underline flex-shrink-0"
+                >
+                  了解更多
+                </button>
+              </div>
+            </div>
+          </div>
        </div>

        <!-- 对话框底部 -->
```

### 3. 方法定义（methods 部分）

**位置**：`src/App.vue` 第 1449-1460 行

```diff
      }
    },
+    async openProductUrl(url) {
+      try {
+        // 使用 Rust 后端命令打开外部链接
+        await invoke('open_url', { url })
+      } catch (err) {
+        console.error('打开链接失败:', err)
+        // 如果 invoke 失败，尝试使用 window.open 作为降级方案
+        if (typeof window !== 'undefined' && window.open) {
+          window.open(url, '_blank')
+        }
+      }
+    },
    async confirmUpdate() {
```

## 设计说明

### 数据结构

- **数组结构**：`otherProducts` 设计为数组，便于后续添加更多产品
- **产品对象字段**：
  - `name`: 产品名称（必填）
  - `description`: 产品简介（必填）
  - `url`: 产品链接（必填）

### UI 风格

- **位置**：在"作者"信息下方，"关闭"按钮上方
- **样式**：
  - 使用 `border-t` 分隔线，与上方内容区分
  - 标题使用 `text-xs text-gray-500`，保持低调
  - 产品名使用 `text-sm font-medium`，稍微加粗但不突出
  - 链接按钮使用 `text-xs text-gray-600 hover:text-gray-900 underline`，文本链接风格
  - 整体风格克制，信息型，不做广告感设计

### 功能实现

- **循环渲染**：使用 `v-for` 循环渲染产品列表
- **条件显示**：使用 `v-if` 确保只在有产品时显示
- **链接打开**：
  - 优先使用 Tauri 的 `invoke('open_url')` 命令
  - 失败时降级使用 `window.open()`（兼容性）

### 扩展性

添加新产品只需在 `otherProducts` 数组中添加新对象：

```javascript
otherProducts: [
  {
    name: 'MeowDocs',
    description: '本地优先的 Markdown 笔记与知识管理工具',
    url: 'https://example.com/meowdocs'
  },
  {
    name: 'Product2',
    description: '产品2的描述',
    url: 'https://example.com/product2'
  }
  // 可以继续添加更多产品
]
```

## 验证要点

1. ✅ 数据结构为数组，便于扩展
2. ✅ UI 使用循环渲染，不硬编码
3. ✅ 样式克制，符合"附加信息"定位
4. ✅ 链接打开使用 Tauri 命令，桌面端兼容
5. ✅ 有降级方案，确保兼容性
6. ✅ 位置正确：作者信息下方，关闭按钮上方

## 使用说明

### 修改产品 URL

只需修改 `otherProducts` 数组中对应产品的 `url` 字段：

```javascript
otherProducts: [
  {
    name: 'MeowDocs',
    description: '本地优先的 Markdown 笔记与知识管理工具',
    url: 'https://meowdocs.com'  // 修改这里
  }
]
```

### 添加新产品

在 `otherProducts` 数组中添加新对象即可：

```javascript
otherProducts: [
  {
    name: 'MeowDocs',
    description: '本地优先的 Markdown 笔记与知识管理工具',
    url: 'https://example.com/meowdocs'
  },
  {
    name: '新产品',
    description: '新产品描述',
    url: 'https://example.com/new-product'
  }
]
```

UI 会自动循环渲染所有产品。

