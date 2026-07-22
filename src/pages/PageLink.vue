<script setup lang="ts">
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import LinkAddonCard from "@/components/LinkAddonCard.vue";

const store = useBlenderAddonStore();
const ui = useUiStore();

const showVersionMenu = ref(false);
const newVersion = ref("");
const detecting = ref(false);
const refreshing = ref(false);
const showUninstallWarn = ref(true);
/** 卡片实例（expose 了 refresh） */
const cardRefs = ref<Array<{ refresh: () => Promise<void> } | null>>([]);

const versionRule = (v: string) => /^\d+\.\d+$/.test(v) || "格式如 4.2";

interface AddonScan {
  path: string;
  is_extension: boolean;
}

async function refreshAll() {
  if (store.addon_list.length === 0) {
    ui.notify("列表是空的，没有可刷新的插件", "info");
    return;
  }
  refreshing.value = true;
  try {
    const cards = cardRefs.value.filter((c): c is { refresh: () => Promise<void> } => !!c);
    await Promise.all(cards.map((c) => c.refresh()));
    ui.ok(`已刷新 ${cards.length} 个插件的链接状态`);
  } catch (e) {
    ui.error(`刷新失败：${e}`);
  } finally {
    refreshing.value = false;
  }
}

async function pickAddonFolder() {
  try {
    const picked = await open({directory: true, multiple: true, title: "选择插件文件夹"});
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    const found = await invoke<AddonScan[]>("scan_addon_paths", {paths});
    if (found.length === 0) {
      ui.notify("所选目录里没有找到插件（需要 __init__.py 或 blender_manifest.toml）", "warning");
      return;
    }
    let added = 0;
    for (const f of found) {
      if (store.add_addon({addon_path: f.path, is_extension: f.is_extension, is_expand: true})) added++;
    }
    ui.notify(added > 0 ? `已添加 ${added} 个插件` : "插件已在列表中", added > 0 ? "success" : "info");
  } catch (e) {
    ui.error(`添加插件失败：${e}`);
  }
}

async function detectVersions() {
  detecting.value = true;
  try {
    const found = await invoke<string[]>("detect_config_versions");
    if (found.length === 0) {
      ui.notify("没有在用户目录里发现 Blender 版本（可能从未启动过 Blender）", "warning");
      return;
    }
    let added = 0;
    for (const v of found) {
      if (!store.blender_version_list.includes(v)) {
        store.add_blender_version(v);
        added++;
      }
    }
    ui.ok(added > 0 ? `检测到 ${found.length} 个版本，新增 ${added} 个` : `检测到 ${found.length} 个版本，均已在列表中`);
  } catch (e) {
    ui.error(`检测失败：${e}`);
  } finally {
    detecting.value = false;
  }
}

function addVersion() {
  const v = newVersion.value.trim();
  if (!/^\d+\.\d+$/.test(v)) return;
  store.add_blender_version(v);
  newVersion.value = "";
  showVersionMenu.value = false;
}
</script>

<template>
  <div class="page-wrap">
    <div class="d-flex align-center mb-5">
      <div>
        <div class="page-title">插件链接</div>
        <div class="page-subtitle">把开发目录链接到各版本 Blender 的插件目录，一份代码多版本调试</div>
      </div>
      <v-spacer/>
      <v-btn
          variant="tonal"
          prepend-icon="mdi-refresh"
          :loading="refreshing"
          :disabled="store.addon_list.length === 0"
          title="重新检查各版本的链接状态"
          @click="refreshAll"
      >
        刷新状态
      </v-btn>
      <v-btn color="primary" variant="flat" prepend-icon="mdi-folder-plus-outline" class="ml-2" @click="pickAddonFolder">
        添加插件
      </v-btn>
    </div>

    <!-- 版本管理 -->
    <v-card class="card-soft mb-4">
      <v-card-text class="d-flex align-center flex-wrap" style="gap: 8px">
        <span class="dim text-body-2 mr-1">Blender 版本</span>
        <v-chip
            v-for="v in store.sorted_versions"
            :key="v"
            size="small"
            variant="tonal"
            label
            class="mono"
            closable
            @click:close="store.remove_blender_version(v)"
        >
          {{ v }}
        </v-chip>

        <v-menu v-model="showVersionMenu" :close-on-content-click="false" location="bottom">
          <template v-slot:activator="{ props }">
            <v-btn v-bind="props" size="small" variant="tonal" icon="mdi-plus" title="手动添加版本"/>
          </template>
          <v-card min-width="240" class="pa-3 card-soft">
            <v-text-field
                v-model="newVersion"
                label="版本号"
                placeholder="如 5.3"
                density="compact"
                hide-details="auto"
                :rules="[versionRule]"
                autofocus
                @keyup.enter="addVersion"
            />
            <v-btn block color="primary" variant="tonal" class="mt-2" size="small" @click="addVersion">添加</v-btn>
          </v-card>
        </v-menu>

        <v-spacer/>
        <v-btn size="small" variant="text" prepend-icon="mdi-magnify-scan" :loading="detecting" @click="detectVersions">
          自动检测
        </v-btn>
        <v-btn size="small" variant="text" prepend-icon="mdi-restore" @click="store.restore_blender_version()">
          恢复默认
        </v-btn>
      </v-card-text>
    </v-card>

    <v-alert
        v-if="showUninstallWarn && store.addon_list.length > 0"
        type="info"
        variant="tonal"
        density="compact"
        class="mb-4"
        closable
        @click:close="showUninstallWarn = false"
    >
      请通过本工具"断开"来移除链接。在 Blender 偏好设置里对链接安装的扩展点"卸载"，可能连同你的开发目录一起删除。
    </v-alert>

    <!-- 插件列表 -->
    <template v-if="store.addon_list.length > 0">
      <div class="d-flex align-center mb-2" style="gap: 4px">
        <span class="dim text-caption">{{ store.addon_list.length }} 个插件</span>
        <v-spacer/>
        <v-btn size="small" variant="text" prepend-icon="mdi-unfold-less-horizontal"
               @click="store.set_all_expand(false)">
          全部折叠
        </v-btn>
        <v-btn size="small" variant="text" prepend-icon="mdi-unfold-more-horizontal"
               @click="store.set_all_expand(true)">
          全部展开
        </v-btn>
      </div>
      <LinkAddonCard
          v-for="addon in store.addon_list"
          :key="addon.addon_path"
          ref="cardRefs"
          :addon="addon"
      />
      <div class="d-flex justify-end mt-2">
        <v-btn size="small" variant="text" color="error" prepend-icon="mdi-playlist-remove"
               @click="store.clear_addon(); ui.ok('已清空插件列表')">
          清空列表
        </v-btn>
      </div>
    </template>
    <div v-else class="empty-state">
      <v-icon icon="mdi-folder-arrow-down-outline" size="52"/>
      <div class="text-h6">把插件文件夹拖到窗口里</div>
      <div class="text-body-2">支持传统插件（__init__.py）和新版扩展（blender_manifest.toml），也可以点右上角「添加插件」选择目录</div>
    </div>
  </div>
</template>
