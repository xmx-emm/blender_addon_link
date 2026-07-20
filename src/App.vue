<script setup lang="ts">
import {computed, onMounted, onUnmounted} from "vue";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {invoke} from "@tauri-apps/api/core";
import useBlenderAddonStore, {useRenderStore, useUiStore} from "@/stores.ts";
import PageLink from "@/pages/PageLink.vue";
import PageStartup from "@/pages/PageStartup.vue";
import PageBlend from "@/pages/PageBlend.vue";
import PageRender from "@/pages/PageRender.vue";
import PageSettings from "@/pages/PageSettings.vue";

const ui = useUiStore();
const store = useBlenderAddonStore();
const render = useRenderStore();

const pages = [
  {id: 'link', title: '插件链接', icon: 'mdi-link-variant'},
  {id: 'startup', title: '启动分析', icon: 'mdi-rocket-launch-outline'},
  {id: 'blend', title: '文件分析', icon: 'mdi-chart-donut'},
  {id: 'render', title: '渲染队列', icon: 'mdi-movie-open-play-outline'},
  {id: 'settings', title: '设置', icon: 'mdi-cog-outline'},
];

const currentPage = computed(() => {
  switch (ui.page) {
    case 'startup': return PageStartup;
    case 'blend': return PageBlend;
    case 'render': return PageRender;
    case 'settings': return PageSettings;
    default: return PageLink;
  }
});

interface AddonScan {
  path: string;
  is_extension: boolean;
}

async function handleDrop(paths: string[]) {
  const blends = paths.filter(p => p.toLowerCase().endsWith('.blend'));
  const others = paths.filter(p => !p.toLowerCase().endsWith('.blend'));

  if (blends.length > 0) {
    ui.dropped_blends = [...blends];
    // 渲染队列页拖入直接加任务，其它页面跳到文件分析
    if (ui.page !== 'render') {
      ui.page = 'blend';
    }
  }
  if (others.length > 0) {
    try {
      const found = await invoke<AddonScan[]>('scan_addon_paths', {paths: others});
      if (found.length === 0) {
        if (blends.length === 0) {
          ui.notify('拖入的目录里没有找到 Blender 插件（需要 __init__.py 或 blender_manifest.toml）', 'warning');
        }
        return;
      }
      let added = 0;
      for (const f of found) {
        if (store.add_addon({addon_path: f.path, is_extension: f.is_extension, is_expand: true})) {
          added++;
        }
      }
      if (blends.length === 0) {
        ui.page = 'link';
      }
      ui.notify(added > 0 ? `已添加 ${added} 个插件` : '插件已在列表中', added > 0 ? 'success' : 'info');
    } catch (e) {
      ui.error(`识别插件失败：${e}`);
    }
  }
}

const unlisteners: UnlistenFn[] = [];
onMounted(async () => {
  render.recover();
  unlisteners.push(await listen<{ paths: string[] }>('tauri://drag-drop', (e) => {
    ui.dragging = false;
    handleDrop(e.payload.paths ?? []);
  }));
  unlisteners.push(await listen('tauri://drag-enter', () => ui.dragging = true));
  unlisteners.push(await listen('tauri://drag-leave', () => ui.dragging = false));
});
onUnmounted(() => unlisteners.forEach(u => u()));
</script>

<template>
  <v-app class="not_select">
    <v-navigation-drawer permanent rail rail-width="76" class="nav-rail">
      <div class="d-flex flex-column align-center pt-3 pb-1">
        <v-avatar color="primary" size="38" rounded="lg">
          <v-icon icon="mdi-blender-software" size="24"/>
        </v-avatar>
      </div>
      <v-list density="compact" nav class="px-1">
        <v-list-item
            v-for="p in pages"
            :key="p.id"
            :active="ui.page === p.id"
            @click="ui.page = p.id"
            class="nav-item"
            rounded="lg"
        >
          <div class="d-flex flex-column align-center py-1">
            <v-badge
                v-if="p.id === 'render' && render.queue_running"
                dot color="primary" offset-x="-2" offset-y="2"
            >
              <v-icon :icon="p.icon" size="22"/>
            </v-badge>
            <v-icon v-else :icon="p.icon" size="22"/>
            <span class="nav-label">{{ p.title }}</span>
          </div>
        </v-list-item>
      </v-list>
    </v-navigation-drawer>

    <v-main class="main-bg">
      <keep-alive>
        <component :is="currentPage"/>
      </keep-alive>
    </v-main>

    <!-- 拖拽遮罩 -->
    <div v-if="ui.dragging" class="drop-overlay">
      <v-icon icon="mdi-tray-arrow-down" size="56"/>
      <div class="text-h6 mt-3">松开添加</div>
      <div class="dim mt-1">插件文件夹 → 插件链接　·　.blend 文件 → 文件分析 / 渲染队列</div>
    </div>

    <v-snackbar
        v-model="ui.snackbar"
        :color="ui.snackbar_color"
        timeout="3200"
        location="bottom right"
        variant="tonal"
    >
      {{ ui.snackbar_text }}
      <template v-slot:actions>
        <v-btn icon="mdi-close" size="x-small" @click="ui.snackbar = false"/>
      </template>
    </v-snackbar>
  </v-app>
</template>

<style scoped>
.nav-rail {
  border-right: 1px solid rgba(255, 255, 255, 0.07);
}

.nav-item {
  margin-bottom: 4px;
  min-height: 56px;
}

.nav-label {
  font-size: 11px;
  margin-top: 3px;
  line-height: 1;
}

.main-bg {
  background: rgb(var(--v-theme-background));
  height: 100vh;
  overflow-y: auto;
}

.drop-overlay {
  position: fixed;
  inset: 10px;
  z-index: 3000;
  border: 2px dashed rgb(var(--v-theme-primary));
  border-radius: 16px;
  background: rgba(22, 22, 25, 0.86);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  color: rgb(var(--v-theme-primary));
}
</style>
