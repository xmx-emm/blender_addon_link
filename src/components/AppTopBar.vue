<script setup lang="ts">
import {WebviewWindow} from '@tauri-apps/api/webviewWindow';
import {TauriEvent} from '@tauri-apps/api/event';
import {computed, onMounted, onUnmounted, ref} from 'vue';
import appIconUrl from '@/assets/app-icon-32.png';

withDefaults(
  defineProps<{
    /** 当前页面名，显示在产品名后面 */
    title?: string;
  }>(),
  {title: ''},
);

const isMaximized = ref(false);

async function updateWindowState() {
  try {
    isMaximized.value = await WebviewWindow.getCurrent().isMaximized();
  } catch {
    // ignore
  }
}

function closeWindow() {
  WebviewWindow.getCurrent().close();
}

function minimizeWindow() {
  WebviewWindow.getCurrent().minimize();
  void updateWindowState();
}

async function toggleMaximize() {
  await WebviewWindow.getCurrent().toggleMaximize();
  await updateWindowState();
}

const maximizeIcon = computed(() =>
  isMaximized.value ? 'mdi-window-restore' : 'mdi-window-maximize',
);

let unlistenResize: (() => void) | null = null;

onMounted(async () => {
  void updateWindowState();
  unlistenResize = await WebviewWindow.getCurrent().listen(TauriEvent.WINDOW_RESIZED, () => {
    void updateWindowState();
  });
});

onUnmounted(() => {
  unlistenResize?.();
});
</script>

<template>
  <!-- 不用 v-system-bar：它会走 Vuetify layout 固定定位，导致下方内容顶到窗口最上沿重叠 -->
  <header data-tauri-drag-region="true" class="app-title-bar">
    <div class="title-bar-brand" data-tauri-drag-region="true">
      <span class="title-bar-app-mark" data-tauri-drag-region="true">
        <img
          class="title-bar-app-icon"
          :src="appIconUrl"
          alt=""
          draggable="false"
          data-tauri-drag-region="true"
        />
      </span>
      <span class="title-bar-product" data-tauri-drag-region="true">Blender Link</span>
      <template v-if="title">
        <span class="title-bar-separator" data-tauri-drag-region="true">/</span>
        <span class="title-bar-label" data-tauri-drag-region="true">{{ title }}</span>
      </template>
    </div>
    <span class="title-bar-drag" data-tauri-drag-region="true"/>
    <div class="title-bar-actions">
      <button type="button" class="title-bar-btn" title="最小化" @click="minimizeWindow">
        <v-icon icon="mdi-window-minimize"/>
      </button>
      <button
        type="button"
        class="title-bar-btn"
        :title="isMaximized ? '还原' : '最大化'"
        @click="toggleMaximize"
      >
        <v-icon :icon="maximizeIcon"/>
      </button>
      <button type="button" class="title-bar-btn title-bar-btn-close" title="关闭" @click="closeWindow">
        <v-icon icon="mdi-window-close"/>
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-title-bar {
  --title-bar-height: 32px;
  --title-bar-icon-size: 20px;
  display: flex;
  align-items: center;
  height: var(--title-bar-height);
  min-height: var(--title-bar-height);
  flex: 0 0 var(--title-bar-height);
  box-sizing: border-box;
  padding-inline-start: 6px;
  background: rgba(var(--v-theme-surface), 0.96);
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  /* 文档流占位，不用 fixed/sticky，避免盖住下方页面标题 */
  position: relative;
  z-index: 100;
  flex-shrink: 0;
  width: 100%;
  user-select: none;
}

.title-bar-brand {
  display: flex;
  align-items: center;
  min-width: 0;
  height: 100%;
  gap: 6px;
  -webkit-app-region: drag;
  app-region: drag;
}

.title-bar-app-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 23px;
  height: 23px;
  flex: 0 0 23px;
  border-radius: 7px;
  background: rgba(var(--v-theme-primary), 0.12);
  pointer-events: none;
}

.title-bar-app-icon {
  width: 19px;
  height: 19px;
  flex: 0 0 19px;
  object-fit: contain;
  pointer-events: none;
  user-select: none;
}

.title-bar-product {
  flex: 0 0 auto;
  color: rgba(var(--v-theme-on-surface), 0.86);
  font-size: 11px;
  font-weight: 680;
  line-height: var(--title-bar-height);
  letter-spacing: -0.01em;
  user-select: none;
}

.title-bar-separator {
  color: rgba(var(--v-theme-on-surface), 0.24);
  font-size: 10px;
  user-select: none;
}

.title-bar-label {
  min-width: 0;
  max-width: 42vw;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 10px;
  font-weight: 560;
  line-height: var(--title-bar-height);
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

.title-bar-drag {
  flex: 1 1 auto;
  min-width: 8px;
  height: 100%;
  -webkit-app-region: drag;
  app-region: drag;
}

.title-bar-actions {
  display: flex;
  height: 100%;
  flex-shrink: 0;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.title-bar-actions .title-bar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--title-bar-height);
  height: var(--title-bar-height);
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  border-radius: 0;
  opacity: 0.8;
  cursor: pointer;
}

.title-bar-actions .title-bar-btn :deep(.v-icon) {
  font-size: var(--title-bar-icon-size);
  width: 1em;
  height: 1em;
  min-width: 0;
}

.title-bar-actions .title-bar-btn:hover {
  opacity: 1;
  background: rgba(128, 128, 128, 0.15);
}

.title-bar-btn.title-bar-btn-close {
  transition:
    background-color 0.2s ease,
    color 0.2s ease,
    opacity 0.2s ease;
}

.title-bar-actions .title-bar-btn.title-bar-btn-close:hover {
  background-color: rgb(232, 17, 35);
  color: rgb(255, 255, 255);
}
</style>
