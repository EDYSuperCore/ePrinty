<template>
  <header class="titlebar">
    <!-- 左：品牌区（可拖拽） -->
    <div class="brand" data-tauri-drag-region>
      <div class="icon-box" aria-hidden="true">
        <img src="/icon.png" alt="ePrinty" class="icon-img" />
      </div>
      <div class="brand-text">
<div class="brand-name wordmark" aria-label="ePrinty">
  <span class="wm-e">e</span><span class="wm-rest">Printy</span>
</div>


        <div class="brand-slogan">
          {{ displayedSlogan }}<span v-if="isTyping" class="cursor">|</span>
        </div>
      </div>
    </div>

    <!-- 中：留白（可拖拽） -->
    <div class="spacer" data-tauri-drag-region></div>

    <!-- 右：业务操作按钮（不可拖拽，先留空） -->
    <div class="actions" data-tauri-drag-region="false">
      <slot name="actions" />
    </div>

    <!-- 右上：窗口按钮（小尺寸，不可拖拽） -->
    <div class="win" data-tauri-drag-region="false">
      <button class="win-btn" type="button" aria-label="Minimize" @click="minimize">—</button>
      <button class="win-btn win-close" type="button" aria-label="Close" @click="close">✕</button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import { appWindow } from "@tauri-apps/api/window";

const isMaximized = ref(false);
const displayedSlogan = ref("");
const isTyping = ref(false);

const fullSlogan = "让打印这件事，简单一点";
let typeWriterTimer: number | null = null;

async function syncMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch {}
}

function typeWriter() {
  isTyping.value = true;
  let index = 0;
  
  function type() {
    if (index < fullSlogan.length) {
      displayedSlogan.value = fullSlogan.substring(0, index + 1);
      index++;
      // 每个字符显示间隔 150ms，可以根据需要调整
      typeWriterTimer = window.setTimeout(type, 150);
    } else {
      // 打字完成后，延迟一下再隐藏光标，然后清空并重新开始
      typeWriterTimer = window.setTimeout(() => {
        isTyping.value = false;
        // 显示完整文本 2 秒后，清空并重新开始
        typeWriterTimer = window.setTimeout(() => {
          displayedSlogan.value = "";
          typeWriter();
        }, 2000);
      }, 500);
    }
  }
  
  type();
}

async function minimize() {
  await appWindow.minimize();
}
async function toggleMaximize() {
  await appWindow.toggleMaximize();
  await syncMaximized();
}
async function close() {
  await appWindow.close();
}

let unlisten: null | (() => void) = null;

onMounted(async () => {
  await syncMaximized();
  try {
    unlisten = await appWindow.onResized(() => void syncMaximized());
  } catch {}
  // 启动打字机效果
  typeWriter();
});

onBeforeUnmount(() => {
  try {
    unlisten?.();
  } catch {}
  // 清理打字机定时器
  if (typeWriterTimer !== null) {
    clearTimeout(typeWriterTimer);
  }
});
</script>

<style scoped>
@font-face {
  font-family: 'LogoFont';
  src: url('/logo-font-2.ttf') format('opentype');
  font-weight: normal;
  font-style: normal;
  font-display: swap;
}

.titlebar {
  position: relative;
  height: 90px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 10px 0 14px;
  background: linear-gradient(135deg, 
    rgba(255, 255, 255, 1) 0%,
    rgba(249, 250, 251, 1) 20%,
    rgba(240, 253, 250, 1) 40%,
    rgba(240, 249, 255, 1) 60%,
    rgba(250, 245, 255, 1) 80%,
    rgba(255, 255, 255, 1) 100%
  );
  background-size: 300% 300%;
  animation: gradientFlow 12s ease infinite;
  border-bottom: 1px solid #e5e7eb;
  user-select: none;
  overflow: hidden;
}

@keyframes gradientFlow {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 320px;
  position: relative;
  z-index: 1;
}

.icon-box {
  width: 48px;
  height: 48px;
  margin-left:10px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
}

.icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
}

.brand-name {
  font-size: 24px;
  font-weight: 600;
  letter-spacing: 1px;
  background: linear-gradient(135deg, 
    #ff6b6b 0%, 
    #4ecdc4 25%, 
    #45b7d1 50%, 
    #f9ca24 75%, 
    #6c5ce7 100%
  );
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  background-size: 200% 200%;
  animation: gradientShift 3s ease infinite;
}

@keyframes gradientShift {
  0%, 100% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
}

.wordmark {
  font-family: 'LogoFont', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
  font-weight: normal;
}

.brand-slogan {
  margin-top: 2px;
  font-size: 14px;
  color: #4b5563;
}

.cursor {
  display: inline-block;
  margin-left: 2px;
  animation: blink 1s infinite;
  color: #4b5563;
}

@keyframes blink {
  0%, 50% {
    opacity: 1;
  }
  51%, 100% {
    opacity: 0;
  }
}


.spacer {
  flex: 1;
  height: 100%;
  position: relative;
  z-index: 1;
}

.actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-right: 20px;
  padding-top:20px;
  -webkit-app-region: no-drag;
  position: relative;
  z-index: 1;
}

.win {
  display: flex;
  align-items: center;
  gap: 4px;
  position: absolute;
  top: 0px;
  right: 0px;
  z-index: 2;
}

.win-btn {
  width: 28px !important;
  height: 22px !important;
  padding: 0 !important;
  border: 1px solid transparent;
  background: transparent;
  cursor: pointer;
  font-size: 12px !important;
  line-height: 1 !important;
  color: #111827;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.win-btn:hover {
  background:rgb(218, 218, 218);
}

.win-close:hover {
  background:rgb(250, 51, 51);
}



</style>
