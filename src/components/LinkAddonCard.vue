<script setup lang="ts">
import {computed, onMounted, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import {AddonItem, AddonMeta} from "@/data.ts";
import {addonIsExtension, compareVersion, getAddonLinkFolder, parseVersion} from "@/utils/addon.ts";

const props = defineProps<{ addon: AddonItem }>();
const store = useBlenderAddonStore();
const ui = useUiStore();

const addonName = computed(() => props.addon.addon_path.split(/[\\/]/).filter(Boolean).pop() ?? "unknown");
const meta = ref<AddonMeta | null>(null);
const displayName = computed(() => meta.value?.name || addonName.value);
/** 插件声明的最低 Blender 版本，取 major.minor */
const minVersion = computed(() => {
  const m = meta.value?.blender_min;
  if (!m) return "";
  const [a, b] = parseVersion(m);
  return a > 0 ? `${a}.${b}` : "";
});

function belowMin(version: string): boolean {
  return !!minVersion.value && compareVersion(version, minVersion.value) < 0;
}

interface Row {
  version: string;
  install_path: string;
  supported: boolean;
  exists: boolean;
  is_link: boolean;
  target: string | null;
  busy: boolean;
}

const rows = ref<Row[]>([]);
const loading = ref(false);
const confirmRow = ref<Row | null>(null);

const linkedCount = computed(() => rows.value.filter(r => r.is_link).length);
const conflictCount = computed(() => rows.value.filter(r => r.exists && !r.is_link).length);

interface LinkStatus {
  exists: boolean;
  is_link: boolean;
  target: string | null;
}

async function refresh() {
  loading.value = true;
  try {
    const versions = store.sorted_versions;
    const built: Row[] = [];
    for (const v of versions) {
      const supported = !props.addon.is_extension || addonIsExtension(v);
      const folder = await getAddonLinkFolder(v, props.addon.is_extension);
      built.push({
        version: v,
        install_path: `${folder}\\${addonName.value}`,
        supported,
        exists: false,
        is_link: false,
        target: null,
        busy: false,
      });
    }
    const statuses = await invoke<LinkStatus[]>("check_link_status", {
      paths: built.map(r => r.install_path),
    });
    statuses.forEach((s, i) => {
      built[i].exists = s.exists;
      built[i].is_link = s.is_link;
      built[i].target = s.target;
    });
    rows.value = built;
  } catch (e) {
    ui.error(`读取链接状态失败：${e}`);
  } finally {
    loading.value = false;
  }
}

async function doLink(row: Row) {
  row.busy = true;
  try {
    await invoke("link_dir", {from: props.addon.addon_path, to: row.install_path});
    ui.ok(`已链接到 Blender ${row.version}`);
  } catch (e) {
    ui.error(String(e));
  } finally {
    row.busy = false;
    await refresh();
  }
}

async function doUnlink(row: Row) {
  row.busy = true;
  try {
    await invoke("unlink_dir", {ud: row.install_path});
    ui.ok(`已断开 Blender ${row.version} 的链接`);
  } catch (e) {
    ui.error(String(e));
  } finally {
    row.busy = false;
    await refresh();
  }
}

/** 目标处是真实目录：确认后删除并替换为链接 */
async function doReplace() {
  const row = confirmRow.value;
  confirmRow.value = null;
  if (!row) return;
  row.busy = true;
  try {
    await invoke("remove_real_dir", {path: row.install_path});
    await invoke("link_dir", {from: props.addon.addon_path, to: row.install_path});
    ui.ok(`已替换为链接（Blender ${row.version}）`);
  } catch (e) {
    ui.error(String(e));
  } finally {
    row.busy = false;
    await refresh();
  }
}

async function linkAll() {
  for (const row of rows.value) {
    if (row.supported && !row.exists) {
      await doLink(row);
    }
  }
}

async function unlinkAll() {
  for (const row of rows.value) {
    if (row.is_link) {
      await doUnlink(row);
    }
  }
}

async function openFolder(path: string) {
  try {
    await invoke("open_in_explorer", {path});
  } catch (e) {
    ui.error(String(e));
  }
}

function statusColor(r: Row): string {
  if (!r.supported) return "grey";
  if (r.is_link) return "success";
  if (r.exists) return "warning";
  return "grey";
}

function statusText(r: Row): string {
  if (!r.supported) return "此版本不支持扩展";
  const base = r.is_link ? "已链接" : r.exists ? "已有同名目录（非链接）" : "未链接";
  return belowMin(r.version) ? `${base}（低于插件要求的 ${minVersion.value}）` : base;
}

async function loadMeta() {
  try {
    meta.value = await invoke<AddonMeta>("read_addon_meta", {path: props.addon.addon_path});
  } catch {
    meta.value = null;
  }
}

onMounted(() => {
  loadMeta();
  refresh();
});
watch(() => store.sorted_versions.join(","), refresh);
defineExpose({refresh});
</script>

<template>
  <v-card class="card-soft mb-3">
    <v-card-text class="pb-2">
      <div class="d-flex align-center" style="gap: 10px">
        <v-avatar :color="addon.is_extension ? 'primary' : 'secondary'" size="34" rounded="lg" variant="tonal">
          <v-icon :icon="addon.is_extension ? 'mdi-puzzle' : 'mdi-language-python'" size="18"/>
        </v-avatar>
        <div style="min-width: 0">
          <div class="d-flex align-center" style="gap: 8px">
            <span class="font-weight-bold text-body-1">{{ displayName }}</span>
            <span v-if="meta?.version" class="dim text-caption mono">v{{ meta.version }}</span>
            <v-chip size="x-small" :color="addon.is_extension ? 'primary' : 'secondary'" variant="tonal" label>
              {{ addon.is_extension ? '扩展 (4.2+)' : '传统插件' }}
            </v-chip>
            <v-chip v-if="minVersion" size="x-small" variant="tonal" label class="mono"
                    title="插件声明的最低 Blender 版本">
              ≥ {{ minVersion }}
            </v-chip>
            <v-chip v-if="linkedCount" size="x-small" color="success" variant="tonal" label>
              已链接 {{ linkedCount }}
            </v-chip>
            <v-chip v-if="conflictCount" size="x-small" color="warning" variant="tonal" label>
              冲突 {{ conflictCount }}
            </v-chip>
          </div>
          <div class="dim text-caption mono text-truncate link-path" @click="openFolder(addon.addon_path)"
               title="在资源管理器中打开">
            {{ addon.addon_path }}
          </div>
        </div>
        <v-spacer/>
        <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-link-variant"
               :disabled="loading" @click="linkAll">全部链接
        </v-btn>
        <v-btn size="small" variant="text" prepend-icon="mdi-link-variant-off"
               :disabled="loading || linkedCount === 0" @click="unlinkAll">全部断开
        </v-btn>
        <v-btn icon="mdi-chevron-down" size="small" variant="text"
               :style="{transform: addon.is_expand ? 'rotate(180deg)' : ''}"
               @click="addon.is_expand = !addon.is_expand"/>
        <v-btn icon="mdi-close" size="small" variant="text" title="从列表移除（不影响磁盘文件）"
               @click="store.remove_addon(addon)"/>
      </div>

      <!-- 收起状态：一行状态点 -->
      <div v-if="!addon.is_expand" class="d-flex align-center mt-2 ml-11" style="gap: 6px">
        <v-tooltip v-for="r in rows" :key="r.version" :text="`${r.version}：${statusText(r)}`">
          <template v-slot:activator="{ props: tp }">
            <v-chip v-bind="tp" size="small" :color="statusColor(r)" variant="tonal" label
                    class="mono" @click="r.supported && (r.is_link ? doUnlink(r) : (r.exists ? confirmRow = r : doLink(r)))">
              {{ r.version }}
              <v-icon end size="14"
                      :icon="!r.supported ? 'mdi-cancel' : r.is_link ? 'mdi-check' : r.exists ? 'mdi-alert' : 'mdi-link-off'"/>
            </v-chip>
          </template>
        </v-tooltip>
      </div>
    </v-card-text>

    <!-- 展开状态：详细行 -->
    <v-expand-transition>
      <div v-if="addon.is_expand">
        <v-divider/>
        <v-list density="compact" class="py-0" bg-color="transparent">
          <v-list-item v-for="r in rows" :key="r.version" class="py-1">
            <template v-slot:prepend>
              <v-chip size="small" variant="tonal" label class="mono mr-3" style="width: 52px; justify-content: center">
                {{ r.version }}
              </v-chip>
            </template>
            <v-list-item-title class="text-body-2 d-flex align-center" style="gap: 8px">
              <v-icon size="15" :color="statusColor(r)"
                      :icon="!r.supported ? 'mdi-cancel' : r.is_link ? 'mdi-check-circle' : r.exists ? 'mdi-alert-circle' : 'mdi-circle-outline'"/>
              <v-icon v-if="r.supported && belowMin(r.version)" icon="mdi-alert-outline" size="14" color="warning"/>
              <span :class="{'dim': !r.supported, 'text-warning': r.supported && belowMin(r.version)}">{{ statusText(r) }}</span>
              <span v-if="r.is_link && r.target" class="dim text-caption mono text-truncate">→ {{ r.target }}</span>
            </v-list-item-title>
            <template v-slot:append>
              <div class="d-flex align-center" style="gap: 4px">
                <v-btn v-if="r.exists" icon="mdi-folder-open-outline" size="x-small" variant="text"
                       title="打开安装位置" @click="openFolder(r.install_path)"/>
                <v-btn v-if="!r.exists && r.supported" size="x-small" variant="tonal" color="primary"
                       :loading="r.busy" @click="doLink(r)">链接
                </v-btn>
                <v-btn v-else-if="r.is_link" size="x-small" variant="tonal"
                       :loading="r.busy" @click="doUnlink(r)">断开
                </v-btn>
                <v-btn v-else-if="r.exists && !r.is_link" size="x-small" variant="tonal" color="warning"
                       :loading="r.busy" @click="confirmRow = r">替换为链接
                </v-btn>
              </div>
            </template>
          </v-list-item>
        </v-list>
      </div>
    </v-expand-transition>

    <!-- 覆盖真实目录的确认 -->
    <v-dialog :model-value="!!confirmRow" max-width="480" @update:model-value="confirmRow = null">
      <v-card v-if="confirmRow" class="card-soft">
        <v-card-title class="text-subtitle-1">替换为链接？</v-card-title>
        <v-card-text>
          Blender {{ confirmRow.version }} 的插件目录下已存在真实目录：
          <div class="mono text-caption my-2">{{ confirmRow.install_path }}</div>
          <v-alert type="warning" variant="tonal" density="compact" class="mt-2">
            该目录及其内容将被删除，然后创建指向开发目录的链接。此操作不可撤销。
          </v-alert>
        </v-card-text>
        <v-card-actions>
          <v-spacer/>
          <v-btn variant="text" @click="confirmRow = null">取消</v-btn>
          <v-btn color="warning" variant="tonal" @click="doReplace">删除并链接</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-card>
</template>

<style scoped>
.link-path {
  cursor: pointer;
  max-width: 560px;
}

.link-path:hover {
  color: rgb(var(--v-theme-primary));
}
</style>
