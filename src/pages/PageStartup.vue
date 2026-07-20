<script setup lang="ts">
import {computed, onMounted, onUnmounted, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import {StartupProgress, StartupResult} from "@/data.ts";
import {formatSeconds} from "@/utils/addon.ts";

const store = useBlenderAddonStore();
const ui = useUiStore();

const version = ref<string>("");
const runs = ref(3);
const running = ref(false);
const progress = ref<StartupProgress | null>(null);
const result = ref<StartupResult | null>(null);

const configured = computed(() => store.sorted_versions.filter(v => !!store.exe_map[v]));

const normalMedian = computed(() => median(result.value?.normal_seconds ?? []));
const factoryMedian = computed(() => median(result.value?.factory_seconds ?? []));
const addonSum = computed(() => (result.value?.addons ?? []).reduce((s, a) => s + a.seconds, 0));
const maxAddonSec = computed(() =>
    Math.max(0.0001, ...(result.value?.addons ?? []).map(a => a.seconds)));

function median(arr: number[]): number {
  if (arr.length === 0) return 0;
  const s = [...arr].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}

function addonColor(sec: number): string {
  if (sec >= 1) return "error";
  if (sec >= 0.3) return "warning";
  return "success";
}

async function analyze() {
  if (!version.value) return;
  const exe = store.exe_map[version.value];
  if (!exe) return;
  running.value = true;
  result.value = null;
  progress.value = null;
  try {
    result.value = await invoke<StartupResult>("startup_analyze", {
      exe,
      version: version.value,
      runs: runs.value,
    });
    ui.ok("启动分析完成");
  } catch (e) {
    if (String(e) !== "已取消") {
      ui.error(`分析失败：${e}`);
    } else {
      ui.notify("已取消分析", "info");
    }
  } finally {
    running.value = false;
    progress.value = null;
  }
}

async function cancel() {
  await invoke("startup_cancel");
}

let unlisten: UnlistenFn | null = null;
onMounted(async () => {
  if (configured.value.length > 0 && !version.value) {
    version.value = configured.value[configured.value.length - 1];
  }
  unlisten = await listen<StartupProgress>("startup-progress", (e) => {
    progress.value = e.payload;
  });
});
onUnmounted(() => unlisten?.());
</script>

<template>
  <div class="page-wrap">
    <div class="d-flex align-center mb-5">
      <div>
        <div class="page-title">启动分析</div>
        <div class="page-subtitle">找出是哪些插件在拖慢 Blender 启动</div>
      </div>
    </div>

    <!-- 控制区 -->
    <v-card class="card-soft mb-4">
      <v-card-text class="d-flex align-center flex-wrap" style="gap: 16px">
        <v-select
            v-model="version"
            :items="configured"
            label="Blender 版本"
            density="compact"
            hide-details
            style="max-width: 180px"
            :disabled="running"
        />
        <v-select
            v-model="runs"
            :items="[1, 2, 3, 4, 5]"
            label="测量轮数"
            density="compact"
            hide-details
            style="max-width: 120px"
            :disabled="running"
        />
        <v-btn v-if="!running" color="primary" variant="flat" prepend-icon="mdi-play"
               :disabled="!version" @click="analyze">
          开始分析
        </v-btn>
        <v-btn v-else color="error" variant="tonal" prepend-icon="mdi-stop" @click="cancel">
          取消
        </v-btn>
        <span class="dim text-caption">
          会多次启动 Blender 后台进程（不弹窗口），插件多时需要几分钟
        </span>
      </v-card-text>
      <v-expand-transition>
        <div v-if="running && progress">
          <v-divider/>
          <v-card-text class="py-3">
            <div class="d-flex align-center mb-2" style="gap: 10px">
              <v-progress-circular indeterminate size="18" width="2" color="primary"/>
              <span class="text-body-2">{{ progress.message }}</span>
              <v-spacer/>
              <span class="dim text-caption">{{ progress.step }} / {{ progress.total }}</span>
            </div>
            <v-progress-linear :model-value="progress.step / progress.total * 100" color="primary" rounded height="6"/>
          </v-card-text>
        </div>
      </v-expand-transition>
    </v-card>

    <template v-if="configured.length === 0">
      <div class="empty-state">
        <v-icon icon="mdi-cog-outline" size="52"/>
        <div class="text-h6">先配置 blender.exe</div>
        <div class="text-body-2">到「设置」页点击「自动探测已安装的 Blender」，或手动浏览选择</div>
        <v-btn class="mt-2" variant="tonal" color="primary" @click="ui.page = 'settings'">去设置</v-btn>
      </div>
    </template>

    <!-- 结果 -->
    <template v-if="result">
      <v-row dense class="mb-1">
        <v-col cols="4">
          <v-card class="card-soft stat-card">
            <div class="stat-value">{{ formatSeconds(normalMedian) }}</div>
            <div class="stat-label">正常启动（含插件与用户设置）</div>
          </v-card>
        </v-col>
        <v-col cols="4">
          <v-card class="card-soft stat-card">
            <div class="stat-value">{{ formatSeconds(factoryMedian) }}</div>
            <div class="stat-label">纯净启动（--factory-startup）</div>
          </v-card>
        </v-col>
        <v-col cols="4">
          <v-card class="card-soft stat-card">
            <div class="stat-value" style="color: rgb(var(--v-theme-primary))">
              +{{ formatSeconds(Math.max(0, normalMedian - factoryMedian)) }}
            </div>
            <div class="stat-label">插件 + 用户配置的额外开销</div>
          </v-card>
        </v-col>
      </v-row>

      <v-card class="card-soft mb-4" v-if="result.addons.length > 0">
        <v-card-title class="text-subtitle-1 d-flex align-center">
          各插件加载耗时
          <v-spacer/>
          <span class="dim text-caption">合计 {{ formatSeconds(addonSum) }} · {{ result.addons.length }} 个插件</span>
        </v-card-title>
        <v-card-text>
          <div v-for="a in result.addons" :key="a.module" class="addon-row">
            <div class="addon-name text-body-2">
              <v-icon v-if="!a.ok" icon="mdi-alert-circle" color="error" size="14" class="mr-1"
                      :title="a.error || '加载失败'"/>
              {{ a.display_name }}
              <span v-if="a.module.startsWith('bl_ext.')" class="dim text-caption ml-1">扩展</span>
            </div>
            <div class="addon-bar-zone">
              <div class="addon-bar" :class="`bg-${addonColor(a.seconds)}`"
                   :style="{width: Math.max(1.5, a.seconds / maxAddonSec * 100) + '%'}"/>
            </div>
            <div class="addon-sec mono text-body-2" :class="`text-${addonColor(a.seconds)}`">
              {{ a.seconds >= 0.01 ? a.seconds.toFixed(2) : '<0.01' }}s
            </div>
          </div>
          <v-alert
              v-if="result.addons.some(a => a.seconds >= 1)"
              type="warning" variant="tonal" density="compact" class="mt-4"
          >
            {{
              result.addons.filter(a => a.seconds >= 1).map(a => a.display_name).join('、')
            }} 加载超过 1 秒——不常用的话，建议在 Blender 偏好设置中禁用，用时再开。
          </v-alert>
        </v-card-text>
      </v-card>

      <v-card class="card-soft">
        <v-card-text class="dim text-body-2">
          <div class="mb-1"><v-icon icon="mdi-information-outline" size="15" class="mr-1"/>说明</div>
          <div v-for="(n, i) in result.notes" :key="i">· {{ n }}</div>
          <div>· 预热运行耗时 {{ formatSeconds(result.warmup_seconds) }}（首轮受杀毒扫描与磁盘缓存影响，未计入统计）</div>
          <div>· 逐插件计时在纯净环境中逐个启用测得，插件间共享依赖时，先加载者会承担公共库的导入时间</div>
        </v-card-text>
      </v-card>
    </template>
  </div>
</template>

<style scoped>
.stat-card {
  padding: 18px 20px;
}

.stat-value {
  font-size: 1.6rem;
  font-weight: 700;
  line-height: 1.1;
}

.stat-label {
  color: rgba(255, 255, 255, 0.55);
  font-size: 0.8rem;
  margin-top: 4px;
}

.addon-row {
  display: grid;
  grid-template-columns: 240px 1fr 72px;
  align-items: center;
  gap: 12px;
  padding: 3px 0;
}

.addon-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.addon-bar-zone {
  background: rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  height: 14px;
  overflow: hidden;
}

.addon-bar {
  height: 100%;
  border-radius: 4px;
  transition: width .4s ease;
}

.addon-sec {
  text-align: right;
}
</style>
