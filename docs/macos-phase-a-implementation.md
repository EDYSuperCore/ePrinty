# macOS Phase A 实现总结

## 目标
在 macOS 上实现 Phase A：保留系统窗体能力（decorations=true），标题栏采用透明风格（titleBarStyle=Transparent），不启用真正透明窗体（transparent=false）；前端新增一个"假标题栏"视觉层，避免内容顶到最上方被系统标题栏覆盖。

## 改动清单

### 1. 新增 platform-specific 配置文件

**文件路径**: `src-tauri/tauri.macos.conf.json`

**覆盖字段**:
```json
{
  "tauri": {
    "windows": [
      {
        "decorations": true,        // 保留系统窗体装饰（三色按钮、系统标题栏）
        "transparent": false,        // 不启用透明窗体
        "titleBarStyle": "Transparent"  // 标题栏透明风格
      }
    ]
  }
}
```

**说明**:
- Tauri v2 CLI 会自动查找并合并 `tauri.macos.conf.json` 与 `tauri.conf.json`
- 仅覆盖窗口相关字段，其他配置继承自主配置文件
- 该配置确保在 macOS 上使用系统窗体，同时标题栏更融合

### 2. 前端"假标题栏"视觉层

**修改文件**: `src/App.vue`

**主要改动**:

#### 2.1 模板层（Template）
- 在根元素 `.app-frame` 添加条件 class：`:class="{ 'macos-with-system-titlebar': isMacOS }"`
- 新增假标题栏容器（仅在 macOS 上显示）：
  ```html
  <div v-if="isMacOS" class="macos-fake-titlebar">
    <div class="fake-titlebar-content">
      <div class="fake-titlebar-title">ePrinty - 让打印这件事，简单一点</div>
    </div>
  </div>
  ```
- Windows 自定义标题栏 `AppTitleBar` 添加条件渲染：`v-if="!isMacOS"`
- 内容区 `.app-content` 添加条件 class：`:class="{ 'macos-content': isMacOS }"`

#### 2.2 数据层（Data）
- 新增平台检测常量：
  ```javascript
  isMacOS: navigator.userAgent.includes('Mac OS X')
  ```

#### 2.3 样式层（Style）
```css
/* 假标题栏样式 */
.macos-fake-titlebar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 52px;  /* 标题栏高度 */
  background: linear-gradient(to bottom, rgba(246, 246, 246, 0.95), rgba(235, 235, 235, 0.92));
  backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  z-index: 9999;
}

/* 为根容器添加顶部边距，防止内容被覆盖 */
.macos-with-system-titlebar {
  padding-top: 52px;
}
```

**关键特性**:
- 假标题栏高度为 52px，与系统标题栏高度协调
- 使用半透明背景和 backdrop-filter 实现毛玻璃效果
- 不使用 `data-tauri-drag-region`，所有窗口拖拽由系统标题栏处理
- 假标题栏仅作为视觉层，不接管任何交互

### 3. Phase B 预埋（不启用）

**已完成**:
- 平台检测常量 `isMacOS` 已预埋，可用于后续 Phase B 扩展
- 当前不引入任何窗口 API（setSize/setPosition/restore）
- 保持代码干净，不添加 Safe Mode、window wrapper、诊断逻辑

## 验收标准

### ✅ 功能验收
1. macOS dev 启动后，显示系统窗体（有系统三色按钮）
2. 标题栏视觉更融合（Transparent 风格）
3. 最大化→还原操作稳定无异常
4. 窗口拖拽正常（系统原生拖拽）
5. 点击/键盘输入稳定无异常

### ✅ 视觉验收
1. 页面顶部显示假标题栏区域（52px 高度）
2. 假标题栏显示应用标题文字
3. 页面内容不被系统标题栏覆盖
4. 假标题栏与系统标题栏视觉融合良好

### ✅ 跨平台兼容性
1. Windows 平台不受影响，继续使用自定义标题栏
2. 平台特定配置文件正确加载
3. 条件渲染逻辑正确执行

## 运行测试

```bash
# macOS 开发模式运行
npm run tauri dev
```

**预期结果**:
- 窗口顶部显示 macOS 系统标题栏（三色按钮、透明背景）
- 内容区顶部显示假标题栏（灰色渐变背景、应用标题）
- 内容不会被标题栏遮挡
- 所有窗口操作（最大化、拖拽、关闭）均正常工作

## 技术细节

### 配置合并机制
Tauri v2 使用 JSON Merge Patch 策略自动合并平台特定配置：
1. 读取 `tauri.conf.json` 作为基础配置
2. 根据当前平台查找 `tauri.{platform}.conf.json`
3. 深度合并平台特定配置，覆盖对应字段

### 平台检测策略
使用 `navigator.userAgent` 检测 macOS：
```javascript
isMacOS: navigator.userAgent.includes('Mac OS X')
```

### 布局策略
- 假标题栏使用 `position: fixed` 固定在顶部
- 根容器使用 `padding-top: 52px` 为假标题栏预留空间
- 确保滚动内容从假标题栏下方开始

## Phase B 展望

Phase A 已为后续 Phase B 预留扩展点：
- 平台检测常量可用于条件启用自定义拖拽
- 假标题栏可以演进为功能性标题栏（添加按钮、拖拽区域）
- 可以逐步引入窗口 API 进行更精细的控制

当前保持简洁，Phase A 完全依赖系统能力，稳定性最佳。
