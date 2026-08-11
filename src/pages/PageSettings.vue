<script setup lang="ts">
import {computed, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import {BlenderExe, CleanupResult, CleanupTarget} from "@/data.ts";
import {formatBytes, getBlenderVersionFolder} from "@/utils/addon.ts";
import packageMeta from "../../package.json";

const store = useBlenderAddonStore();
const ui = useUiStore();
const appVersion = packageMeta.version;
const detecting = ref(false);
const launching = ref<string | null>(null);

// ---- 磁盘清理 ----
const scanning = ref(false);
const cleaning = ref(false);
const targets = ref<CleanupTarget[] | null>(null);
const selected = ref<string[]>([]);
const confirmClean = ref(false);

const selectedBytes = computed(() =>
    (targets.value ?? []).filter(t => selected.value.includes(t.id)).reduce((s, t) => s + t.bytes, 0));

async function scanDisk() {
  scanning.value = true;
  try {
    targets.value = await invoke<CleanupTarget[]>("scan_cleanup");
    selected.value = targets.value.map(t => t.id);
    if (targets.value.length === 0) {
      ui.ok("很干净，没有可清理的内容");
    }
  } catch (e) {
    ui.error(`扫描失败：${e}`);
  } finally {
    scanning.value = false;
  }
}

async function doClean() {
  confirmClean.value = false;
  cleaning.value = true;
  try {
    const res = await invoke<CleanupResult>("run_cleanup", {ids: selected.value});
    ui.ok(`已清理 ${res.deleted} 个文件，释放 ${formatBytes(res.freed)}`);
    if (res.errors.length > 0) {
      ui.notify(`有 ${res.errors.length} 项跳过（可能被占用，关闭 Blender 后重试）`, "warning");
    }
    await scanDisk();
  } catch (e) {
    ui.error(`清理失败：${e}`);
  } finally {
    cleaning.value = false;
  }
}

async function autoDetect() {
  detecting.value = true;
  try {
    const found = await invoke<BlenderExe[]>("detect_blender_executables");
    if (found.length === 0) {
      ui.notify("没有自动找到 blender.exe，可以手动浏览选择", "warning");
      return;
    }
    let filled = 0;
    for (const f of found) {
      if (!store.blender_version_list.includes(f.version)) {
        store.add_blender_version(f.version);
      }
      if (!store.exe_map[f.version]) {
        store.set_exe(f.version, f.path);
        filled++;
      }
    }
    ui.ok(`找到 ${found.length} 个 Blender（${found.map(f => `${f.version} · ${f.source}`).join('，')}），填充 ${filled} 项`);
  } catch (e) {
    ui.error(`探测失败：${e}`);
  } finally {
    detecting.value = false;
  }
}

async function browseExe(version: string) {
  try {
    const picked = await open({
      title: `选择 Blender ${version} 的 blender.exe`,
      multiple: false,
      filters: [{name: "Blender", extensions: ["exe"]}],
    });
    if (!picked || Array.isArray(picked)) return;
    const detected = await invoke<string>("probe_blender_exe", {path: picked});
    if (detected !== version) {
      ui.notify(`所选文件是 Blender ${detected}，已填到对应版本`, "warning");
      if (!store.blender_version_list.includes(detected)) {
        store.add_blender_version(detected);
      }
      store.set_exe(detected, picked);
    } else {
      store.set_exe(version, picked);
      ui.ok(`已设置 Blender ${version} 的可执行文件`);
    }
  } catch (e) {
    ui.error(String(e));
  }
}

async function launch(version: string) {
  const exe = store.exe_map[version];
  if (!exe) return;
  launching.value = version;
  try {
    await invoke("launch_blender", {exe});
    ui.ok(`已启动 Blender ${version}`);
  } catch (e) {
    ui.error(String(e));
  } finally {
    launching.value = null;
  }
}

async function openConfigDir(version: string) {
  try {
    await invoke("open_in_explorer", {path: await getBlenderVersionFolder(version)});
  } catch (e) {
    ui.error(`打开失败：${e}（该版本可能还没启动过）`);
  }
}
</script>

<template>
  <div class="page-wrap">
    <div class="d-flex align-center mb-5">
      <div>
        <div class="page-title">设置</div>
        <div class="page-subtitle">为每个版本配置 blender.exe —— 启动分析与渲染队列需要用到</div>
      </div>
      <v-spacer/>
      <v-btn color="primary" variant="flat" prepend-icon="mdi-magnify-scan" :loading="detecting" @click="autoDetect">
        自动探测已安装的 Blender
      </v-btn>
    </div>

    <v-card class="card-soft mb-4">
      <v-list bg-color="transparent" density="comfortable">
        <template v-for="(v, i) in store.sorted_versions" :key="v">
          <v-list-item>
            <template v-slot:prepend>
              <v-chip variant="tonal" label class="mono mr-4" style="width: 56px; justify-content: center">
                {{ v }}
              </v-chip>
            </template>
            <v-list-item-title>
              <span v-if="store.exe_map[v]" class="mono text-body-2">{{ store.exe_map[v] }}</span>
              <span v-else class="dim text-body-2">未配置 blender.exe</span>
            </v-list-item-title>
            <template v-slot:append>
              <div class="d-flex align-center" style="gap: 4px">
                <v-btn size="small" variant="tonal" prepend-icon="mdi-file-find-outline" @click="browseExe(v)">
                  浏览
                </v-btn>
                <v-btn size="small" variant="text" icon="mdi-play" title="启动这个版本"
                       :disabled="!store.exe_map[v]" :loading="launching === v" @click="launch(v)"/>
                <v-btn size="small" variant="text" icon="mdi-folder-cog-outline" title="打开配置目录"
                       @click="openConfigDir(v)"/>
                <v-btn v-if="store.exe_map[v]" size="small" variant="text" icon="mdi-close" title="清除路径"
                       @click="store.set_exe(v, '')"/>
              </div>
            </template>
          </v-list-item>
          <v-divider v-if="i < store.sorted_versions.length - 1"/>
        </template>
      </v-list>
    </v-card>

    <!-- 磁盘清理 -->
    <v-card class="card-soft mb-4">
      <v-card-title class="text-subtitle-1 d-flex align-center">
        磁盘清理
        <v-spacer/>
        <v-btn size="small" variant="tonal" prepend-icon="mdi-magnify" :loading="scanning" @click="scanDisk">
          {{ targets === null ? '扫描' : '重新扫描' }}
        </v-btn>
      </v-card-title>
      <v-card-text v-if="targets === null" class="dim text-body-2 pt-0">
        扫描各版本的自动保存文件、资产库索引缓存和 %TEMP% 里的 Blender 临时文件，勾选后一键清理。清理项都可以被 Blender 自动重建，不影响你的工程文件。
      </v-card-text>
      <template v-else-if="targets.length > 0">
        <v-list density="compact" bg-color="transparent" class="py-0">
          <v-list-item v-for="t in targets" :key="t.id" @click="
              selected.includes(t.id) ? selected = selected.filter(i => i !== t.id) : selected.push(t.id)">
            <template v-slot:prepend>
              <v-checkbox-btn :model-value="selected.includes(t.id)" density="compact"/>
            </template>
            <v-list-item-title class="text-body-2">{{ t.label }}</v-list-item-title>
            <v-list-item-subtitle class="mono text-caption">{{ t.path }}</v-list-item-subtitle>
            <template v-slot:append>
              <span class="mono text-body-2 mr-2">{{ formatBytes(t.bytes) }}</span>
              <span class="dim text-caption">{{ t.files }} 个文件</span>
            </template>
          </v-list-item>
        </v-list>
        <v-card-actions>
          <span class="dim text-caption ml-2">自动保存文件清理前请确认没有需要恢复的崩溃现场</span>
          <v-spacer/>
          <v-btn color="primary" variant="tonal" prepend-icon="mdi-broom" :loading="cleaning"
                 :disabled="selected.length === 0" @click="confirmClean = true">
            清理选中（{{ formatBytes(selectedBytes) }}）
          </v-btn>
        </v-card-actions>
      </template>
      <v-card-text v-else class="dim text-body-2 pt-0">没有发现可清理的内容。</v-card-text>
    </v-card>

    <v-card class="card-soft">
      <v-card-text class="dim text-body-2">
        <div class="d-flex align-center mb-1" style="gap: 8px">
          <v-icon icon="mdi-blender-software" color="primary" size="18"/>
          <span class="font-weight-bold" style="color: rgba(255,255,255,.85)">Blender Link 工具箱 v{{ appVersion }}</span>
        </div>
        插件多版本链接 · 启动时间分析 · .blend 文件体积分析 · 渲染队列。
        链接使用 NTFS junction，不需要管理员权限；断开链接只删除链接本身，不会动你的源码目录。
      </v-card-text>
    </v-card>

    <v-dialog v-model="confirmClean" max-width="440">
      <v-card class="card-soft">
        <v-card-title class="text-subtitle-1">确认清理？</v-card-title>
        <v-card-text class="text-body-2">
          将删除选中的 {{ selected.length }} 类内容，共 {{ formatBytes(selectedBytes) }}。
          自动保存文件删除后无法用于崩溃恢复，缓存会在下次使用时自动重建。
        </v-card-text>
        <v-card-actions>
          <v-spacer/>
          <v-btn variant="text" @click="confirmClean = false">取消</v-btn>
          <v-btn color="error" variant="tonal" @click="doClean">清理</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
