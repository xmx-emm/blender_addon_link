<script setup lang="ts">
import {computed, onMounted, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import {AddonItem, AddonMeta} from "@/data.ts";
import {addonIsExtension, compareVersion, getAddonInstallLocations, parseVersion} from "@/utils/addon.ts";

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

interface LocStatus {
  exists: boolean;
  is_link: boolean;
  matches_source: boolean;
  target: string | null;
}

interface Row {
  version: string;
  /** 按插件形态的主安装路径 */
  install_path: string;
  /** 4.2+ 另一形态路径（addons ↔ extensions） */
  alt_path: string | null;
  supported: boolean;
  exists: boolean;
  /** 主路径链接且指向本插件 */
  is_link: boolean;
  /** 主路径是链接但指向其他目录 */
  is_link_other: boolean;
  target: string | null;
  alt_exists: boolean;
  alt_is_link: boolean;
  alt_is_link_other: boolean;
  alt_target: string | null;
  /** addons 与 extensions 同时存在同名安装 */
  dual_install: boolean;
  busy: boolean;
}

const rows = ref<Row[]>([]);
const loading = ref(false);
const confirmRow = ref<Row | null>(null);

const linkedCount = computed(() => rows.value.filter(r => r.is_link || r.alt_is_link).length);
const dualCount = computed(() => rows.value.filter(r => r.dual_install).length);
/** 非双重安装的冲突：指向其他路径 / 真实同名目录 / 另一位置占用 */
const conflictCount = computed(() =>
    rows.value.filter(r =>
        !r.dual_install && (
            r.is_link_other
            || r.alt_is_link_other
            || (r.exists && !r.is_link && !r.is_link_other)
            || (r.alt_exists && !r.alt_is_link && !r.alt_is_link_other && !r.is_link)
        ),
    ).length,
);

interface LinkStatus {
  exists: boolean;
  is_link: boolean;
  matches_source: boolean;
  target: string | null;
}

function emptyLoc(): LocStatus {
  return {exists: false, is_link: false, matches_source: false, target: null};
}

function applyLoc(s: LinkStatus | undefined): LocStatus {
  if (!s) return emptyLoc();
  return {
    exists: s.exists,
    is_link: s.is_link,
    matches_source: s.matches_source,
    target: s.target,
  };
}

async function refresh() {
  loading.value = true;
  try {
    await loadMeta();
    const versions = store.sorted_versions;
    const built: Row[] = [];
    const paths: string[] = [];
    const pathIndex: {primary: number; alt: number | null}[] = [];

    for (const v of versions) {
      const supported = !props.addon.is_extension || addonIsExtension(v);
      const locs = await getAddonInstallLocations(v, addonName.value, props.addon.is_extension);
      const primaryIdx = paths.length;
      paths.push(locs.primary);
      let altIdx: number | null = null;
      if (locs.alternate) {
        altIdx = paths.length;
        paths.push(locs.alternate);
      }
      pathIndex.push({primary: primaryIdx, alt: altIdx});
      built.push({
        version: v,
        install_path: locs.primary,
        alt_path: locs.alternate,
        supported,
        exists: false,
        is_link: false,
        is_link_other: false,
        target: null,
        alt_exists: false,
        alt_is_link: false,
        alt_is_link_other: false,
        alt_target: null,
        dual_install: false,
        busy: false,
      });
    }

    const statuses = await invoke<LinkStatus[]>("check_link_status", {
      paths,
      expectedSource: props.addon.addon_path,
    });

    built.forEach((row, i) => {
      const idx = pathIndex[i];
      const primary = applyLoc(statuses[idx.primary]);
      const alt = idx.alt != null ? applyLoc(statuses[idx.alt]) : emptyLoc();

      row.exists = primary.exists;
      row.is_link = primary.is_link && primary.matches_source;
      row.is_link_other = primary.is_link && !primary.matches_source;
      row.target = primary.target;
      row.alt_exists = alt.exists;
      row.alt_is_link = alt.is_link && alt.matches_source;
      row.alt_is_link_other = alt.is_link && !alt.matches_source;
      row.alt_target = alt.target;
      row.dual_install = primary.exists && alt.exists;
    });
    rows.value = built;
  } catch (e) {
    ui.error(`读取链接状态失败：${e}`);
  } finally {
    loading.value = false;
  }
}

async function doLink(row: Row) {
  // Do not create a second installation when the alternate 4.2+ location is occupied.
  if (!row.supported || row.exists || row.alt_exists) {
    if (row.alt_exists && !row.exists) {
      ui.notify(`Blender ${row.version} 的另一插件位置已有安装，未创建双重安装`, "warning");
    }
    return;
  }
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
    // 断开指向本插件的链接：主路径优先，否则另一位置
    const path = row.is_link
        ? row.install_path
        : (row.alt_is_link && row.alt_path ? row.alt_path : row.install_path);
    await invoke("unlink_dir", {ud: path});
    ui.ok(`已断开 Blender ${row.version} 的链接`);
  } catch (e) {
    ui.error(String(e));
  } finally {
    row.busy = false;
    await refresh();
  }
}

/** 目标处是真实目录或指向其他路径的链接：确认后删除并替换为指向本插件的链接 */
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
  let skipped = 0;
  for (const row of rows.value) {
    if (row.supported && !row.exists && !row.alt_exists) {
      await doLink(row);
    } else if (row.supported && !row.exists && row.alt_exists) {
      skipped++;
    }
  }
  if (skipped > 0) {
    ui.notify(`${skipped} 个版本的另一插件位置已有安装，已跳过以避免双重安装`, "warning");
  }
}

async function unlinkAll() {
  for (const row of rows.value) {
    if (row.is_link || row.alt_is_link) {
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
  if (r.dual_install) return "error";
  if (r.is_link || r.alt_is_link) return "success";
  if (r.is_link_other || r.alt_is_link_other || r.exists || r.alt_exists) return "warning";
  return "grey";
}

function statusIcon(r: Row): string {
  if (!r.supported) return "mdi-cancel";
  if (r.dual_install) return "mdi-alert-octagon";
  if (r.is_link || r.alt_is_link) return "mdi-check-circle";
  if (r.is_link_other || r.alt_is_link_other) return "mdi-link-variant-off";
  if (r.exists || r.alt_exists) return "mdi-alert-circle";
  return "mdi-circle-outline";
}

function statusIconCompact(r: Row): string {
  if (!r.supported) return "mdi-cancel";
  if (r.dual_install) return "mdi-alert-octagon";
  if (r.is_link || r.alt_is_link) return "mdi-check";
  if (r.is_link_other || r.alt_is_link_other || r.exists || r.alt_exists) return "mdi-alert";
  return "mdi-link-off";
}

function statusText(r: Row): string {
  if (!r.supported) return "此版本不支持扩展";
  let base: string;
  if (r.dual_install) {
    const parts: string[] = [];
    if (r.is_link) parts.push("主位置已链接本插件");
    else if (r.is_link_other) parts.push("主位置指向其他路径");
    else if (r.exists) parts.push("主位置有同名目录");
    if (r.alt_is_link) parts.push("另一位置已链接本插件");
    else if (r.alt_is_link_other) parts.push("另一位置指向其他路径");
    else if (r.alt_exists) parts.push("另一位置有同名目录");
    base = parts.length ? `双重安装（${parts.join("；")}）` : "双重安装";
  } else if (r.is_link) {
    base = "已链接";
  } else if (r.is_link_other) {
    base = "指向其他路径";
  } else if (r.exists) {
    base = "已有同名目录（非链接）";
  } else if (r.alt_is_link) {
    base = "已链接（另一位置）";
  } else if (r.alt_is_link_other) {
    base = "另一位置指向其他路径";
  } else if (r.alt_exists) {
    base = "另一位置已有安装";
  } else {
    base = "未链接";
  }
  return belowMin(r.version) ? `${base}（低于插件要求的 ${minVersion.value}）` : base;
}

function canUnlink(r: Row): boolean {
  return r.is_link || r.alt_is_link;
}

function needsReplace(r: Row): boolean {
  return r.exists && !r.is_link;
}

async function loadMeta() {
  try {
    meta.value = await invoke<AddonMeta>("read_addon_meta", {path: props.addon.addon_path});
  } catch {
    meta.value = null;
  }
}

function toggleExpand() {
  props.addon.is_expand = !props.addon.is_expand;
}

/** 收起态点击版本 chip：链接 / 断开 / 替换 */
function onChipClick(r: Row) {
  if (!r.supported || r.busy) return;
  if (canUnlink(r) && !r.dual_install && !r.is_link_other) doUnlink(r);
  else if (needsReplace(r) || r.dual_install) confirmRow.value = r;
  else if (!r.exists) doLink(r);
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
    <!-- 收起：名称摘要 + 便利链接 chips（可点链接/断开） -->
    <v-card-text v-if="!addon.is_expand" class="py-3">
      <div class="d-flex align-center" style="gap: 8px">
        <v-avatar :color="addon.is_extension ? 'primary' : 'secondary'" size="28" rounded="lg" variant="tonal"
                  class="cursor-pointer shrink-0" title="展开" @click="toggleExpand">
          <v-icon :icon="addon.is_extension ? 'mdi-puzzle' : 'mdi-language-python'" size="16"/>
        </v-avatar>
        <span class="font-weight-medium text-body-2 text-truncate cursor-pointer" style="min-width: 0"
              title="展开" @click="toggleExpand">{{ displayName }}</span>
        <v-chip v-if="linkedCount" size="x-small" color="success" variant="tonal" label class="shrink-0">
          已链接 {{ linkedCount }}
        </v-chip>
        <v-chip v-else size="x-small" variant="tonal" label class="shrink-0 dim">未链接</v-chip>
        <v-chip v-if="dualCount" size="x-small" color="error" variant="tonal" label class="shrink-0">
          双重安装 {{ dualCount }}
        </v-chip>
        <v-chip v-if="conflictCount" size="x-small" color="warning" variant="tonal" label class="shrink-0">
          冲突 {{ conflictCount }}
        </v-chip>
        <v-spacer/>
        <v-btn icon="mdi-chevron-down" size="small" variant="text" title="展开" @click="toggleExpand"/>
        <v-btn icon="mdi-close" size="small" variant="text" title="从列表移除（不影响磁盘文件）"
               @click="store.remove_addon(addon)"/>
      </div>
      <div class="d-flex align-center flex-wrap mt-2 ml-9" style="gap: 6px">
        <v-tooltip v-for="r in rows" :key="r.version" :text="`${r.version}：${statusText(r)}`">
          <template v-slot:activator="{ props: tp }">
            <v-chip
                v-bind="tp"
                size="small"
                :color="statusColor(r)"
                variant="tonal"
                label
                class="mono"
                :disabled="r.busy"
                @click="onChipClick(r)"
            >
              {{ r.version }}
              <v-icon end size="14" :icon="statusIconCompact(r)"/>
            </v-chip>
          </template>
        </v-tooltip>
      </div>
    </v-card-text>

    <!-- 展开：完整卡片头 + 版本明细 -->
    <template v-else>
      <v-card-text class="pb-2">
        <div class="d-flex align-center" style="gap: 10px">
          <v-avatar :color="addon.is_extension ? 'primary' : 'secondary'" size="34" rounded="lg" variant="tonal"
                    class="cursor-pointer" title="收起" @click="toggleExpand">
            <v-icon :icon="addon.is_extension ? 'mdi-puzzle' : 'mdi-language-python'" size="18"/>
          </v-avatar>
          <div style="min-width: 0">
            <div class="d-flex align-center flex-wrap" style="gap: 8px">
              <span class="font-weight-bold text-body-1 cursor-pointer" title="收起" @click="toggleExpand">
                {{ displayName }}
              </span>
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
              <v-chip v-if="dualCount" size="x-small" color="error" variant="tonal" label>
                双重安装 {{ dualCount }}
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
          <v-btn size="small" variant="text" icon="mdi-refresh" title="刷新此插件的链接状态"
                 :loading="loading" @click="refresh"/>
          <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-link-variant"
                 :disabled="loading" @click="linkAll">全部链接
          </v-btn>
          <v-btn size="small" variant="text" prepend-icon="mdi-link-variant-off"
                 :disabled="loading || linkedCount === 0" @click="unlinkAll">全部断开
          </v-btn>
          <v-btn icon="mdi-chevron-up" size="small" variant="text" title="收起" @click="toggleExpand"/>
          <v-btn icon="mdi-close" size="small" variant="text" title="从列表移除（不影响磁盘文件）"
                 @click="store.remove_addon(addon)"/>
        </div>
      </v-card-text>

      <v-divider/>
      <v-list density="compact" class="py-0" bg-color="transparent">
        <v-list-item v-for="r in rows" :key="r.version" class="py-1">
          <template v-slot:prepend>
            <v-chip size="small" variant="tonal" label class="mono mr-3" style="width: 52px; justify-content: center">
              {{ r.version }}
            </v-chip>
          </template>
          <v-list-item-title class="text-body-2 d-flex align-center flex-wrap" style="gap: 8px">
            <v-icon size="15" :color="statusColor(r)" :icon="statusIcon(r)"/>
            <v-icon v-if="r.supported && belowMin(r.version)" icon="mdi-alert-outline" size="14" color="warning"/>
            <span :class="{'dim': !r.supported, 'text-warning': r.supported && belowMin(r.version) && !r.dual_install, 'text-error': r.dual_install}">
              {{ statusText(r) }}
            </span>
            <span v-if="(r.is_link || r.is_link_other) && r.target" class="dim text-caption mono text-truncate">
              → {{ r.target }}
            </span>
            <span v-if="r.dual_install && r.alt_path" class="dim text-caption mono text-truncate"
                  :title="`另一位置：${r.alt_path}`">
              ＋ {{ r.alt_path }}
            </span>
          </v-list-item-title>
          <template v-slot:append>
            <div class="d-flex align-center" style="gap: 4px">
              <v-btn v-if="r.exists" icon="mdi-folder-open-outline" size="x-small" variant="text"
                     title="打开主安装位置" @click="openFolder(r.install_path)"/>
              <v-btn v-if="r.alt_exists && r.alt_path" icon="mdi-folder-multiple-outline" size="x-small" variant="text"
                     title="打开另一安装位置" @click="openFolder(r.alt_path)"/>
              <v-btn v-if="!r.exists && !r.alt_exists && r.supported" size="x-small" variant="tonal" color="primary"
                     :loading="r.busy" @click="doLink(r)">链接
              </v-btn>
              <v-btn v-else-if="canUnlink(r) && !r.dual_install && !needsReplace(r)" size="x-small" variant="tonal"
                     :loading="r.busy" @click="doUnlink(r)">断开
              </v-btn>
              <v-btn v-else-if="needsReplace(r) || r.dual_install" size="x-small" variant="tonal"
                     :color="r.dual_install ? 'error' : 'warning'"
                     :loading="r.busy" @click="confirmRow = r">
                {{ r.is_link_other || r.dual_install ? '替换主位置' : '替换为链接' }}
              </v-btn>
            </div>
          </template>
        </v-list-item>
      </v-list>
    </template>

    <!-- 覆盖真实目录 / 其他路径链接 / 双重安装的确认 -->
    <v-dialog :model-value="!!confirmRow" max-width="520" @update:model-value="confirmRow = null">
      <v-card v-if="confirmRow" class="card-soft">
        <v-card-title class="text-subtitle-1">
          {{ confirmRow.dual_install ? '处理双重安装？' : confirmRow.is_link_other ? '替换其他路径的链接？' : '替换为链接？' }}
        </v-card-title>
        <v-card-text>
          <template v-if="confirmRow.dual_install">
            Blender {{ confirmRow.version }} 在传统插件与扩展目录下同时存在同名安装：
            <div class="mono text-caption my-2">主位置：{{ confirmRow.install_path }}</div>
            <div v-if="confirmRow.alt_path" class="mono text-caption mb-2">另一位置：{{ confirmRow.alt_path }}</div>
            <v-alert type="error" variant="tonal" density="compact" class="mt-2">
              「替换主位置」只会删除主位置上的目录/链接，再创建指向本插件的链接；另一位置需你手动处理，避免 Blender 加载两份。
            </v-alert>
          </template>
          <template v-else-if="confirmRow.is_link_other">
            Blender {{ confirmRow.version }} 的安装位置已有链接，但指向其他目录：
            <div class="mono text-caption my-2">{{ confirmRow.install_path }}</div>
            <div v-if="confirmRow.target" class="mono text-caption mb-2">当前指向：{{ confirmRow.target }}</div>
            <v-alert type="warning" variant="tonal" density="compact" class="mt-2">
              将断开该链接，再创建指向本插件开发目录的链接。不会删除被指向的源码目录。
            </v-alert>
          </template>
          <template v-else>
            Blender {{ confirmRow.version }} 的插件目录下已存在真实目录：
            <div class="mono text-caption my-2">{{ confirmRow.install_path }}</div>
            <v-alert type="warning" variant="tonal" density="compact" class="mt-2">
              该目录及其内容将被删除，然后创建指向开发目录的链接。此操作不可撤销。
            </v-alert>
          </template>
        </v-card-text>
        <v-card-actions>
          <v-spacer/>
          <v-btn variant="text" @click="confirmRow = null">取消</v-btn>
          <v-btn :color="confirmRow.dual_install ? 'error' : 'warning'" variant="tonal" @click="doReplace">
            {{ confirmRow.is_link_other || confirmRow.dual_install ? '替换主位置' : '删除并链接' }}
          </v-btn>
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

.cursor-pointer {
  cursor: pointer;
}

.shrink-0 {
  flex-shrink: 0;
}
</style>
