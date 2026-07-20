<script setup lang="ts">
import {computed, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import useBlenderAddonStore, {useUiStore} from "@/stores.ts";
import {BlendAnalysis, PurgeResult} from "@/data.ts";
import {categoryColor, formatBytes, thumbToDataUrl} from "@/utils/addon.ts";

const ui = useUiStore();
const store = useBlenderAddonStore();
const loading = ref(false);
const result = ref<BlendAnalysis | null>(null);
const showAllBlocks = ref(false);

// ---- 孤立数据清理 ----
const purgeDialog = ref(false);
const purging = ref(false);
const purgeVersion = ref("");
const configured = computed(() => store.sorted_versions.filter(v => !!store.exe_map[v]));
const purgeVersionMismatch = computed(() =>
    !!result.value && !!purgeVersion.value && purgeVersion.value !== result.value.blender_version);

function openPurgeDialog() {
  if (!result.value) return;
  if (configured.value.length === 0) {
    ui.notify("请先到设置页配置 blender.exe", "warning");
    return;
  }
  purgeVersion.value = configured.value.includes(result.value.blender_version)
      ? result.value.blender_version
      : configured.value[configured.value.length - 1];
  purgeDialog.value = true;
}

async function doPurge() {
  if (!result.value) return;
  const exe = store.exe_map[purgeVersion.value];
  if (!exe) return;
  purgeDialog.value = false;
  purging.value = true;
  const path = result.value.file;
  try {
    const res = await invoke<PurgeResult>("purge_orphans", {exe, path});
    const delta = res.old_size - res.new_size;
    ui.ok(`已清除 ${res.removed} 个孤立数据块，${formatBytes(res.old_size)} → ${formatBytes(res.new_size)}`
        + (delta > 0 ? `（省 ${formatBytes(delta)}）` : ""));
    await analyzePath(path);
  } catch (e) {
    ui.error(String(e));
  } finally {
    purging.value = false;
  }
}

const fileName = computed(() => result.value?.file.split(/[\\/]/).pop() ?? "");
const thumbUrl = computed(() => {
  const t = result.value?.thumbnail;
  return t ? thumbToDataUrl(t.width, t.height, t.rgba_base64) : null;
});
const totalCatBytes = computed(() =>
    Math.max(1, (result.value?.categories ?? []).reduce((s, c) => s + c.bytes, 0)));

/** 堆叠条与图例：取前 9 类，其余合并为"其他" */
const chartCats = computed(() => {
  const cats = result.value?.categories ?? [];
  const top = cats.slice(0, 9);
  const rest = cats.slice(9);
  const restBytes = rest.reduce((s, c) => s + c.bytes, 0);
  const list = top.map(c => ({
    code: c.code,
    label: c.label,
    bytes: c.bytes,
    count: c.count,
    color: categoryColor(c.code),
  }));
  if (restBytes > 0) {
    list.push({code: "__rest", label: "其他", bytes: restBytes, count: rest.reduce((s, c) => s + c.count, 0), color: "#616161"});
  }
  return list;
});

const shownBlocks = computed(() => {
  const blocks = result.value?.top_blocks ?? [];
  return showAllBlocks.value ? blocks : blocks.slice(0, 15);
});

const suggestions = computed(() => {
  const r = result.value;
  if (!r) return [];
  const out: string[] = [];
  const cat = (code: string) => r.categories.find(c => c.code === code)?.bytes ?? 0;
  const total = totalCatBytes.value;
  const imRatio = cat("IM") / total;
  if (imRatio > 0.45) {
    out.push(`图像占了 ${(imRatio * 100).toFixed(0)}% 的体积——多半是打包贴图。可在 Blender 里「文件 → 外部数据 → 解包」改为外链，或压缩过大的贴图分辨率。`);
  }
  const meRatio = cat("ME") / total;
  if (meRatio > 0.5) {
    out.push(`网格数据占 ${(meRatio * 100).toFixed(0)}%——检查是否有未应用的高细分/雕刻多级细分，或用「清理 → 孤立数据」清掉没用的网格。`);
  }
  if (r.compression === "无压缩") {
    out.push("文件未压缩。在 Blender 保存对话框勾选「压缩」通常能显著减小体积（3.0+ 使用 zstd，速度损失很小）。");
  }
  const dataRatio = cat("DATA") / total;
  if (dataRatio > 0.3) {
    out.push("大量数据无法归属到具体资源（游离附属数据较多），可尝试在 Blender 里执行「清理 → 递归清除孤立数据」后另存。");
  }
  if (out.length === 0) {
    out.push("体积分布正常，没有发现明显可优化的点。");
  }
  return out;
});

async function analyzePath(path: string) {
  loading.value = true;
  try {
    result.value = await invoke<BlendAnalysis>("analyze_blend", {path});
    showAllBlocks.value = false;
    ui.ok(`分析完成：${path.split(/[\\/]/).pop()}`);
  } catch (e) {
    ui.error(`分析失败：${e}`);
  } finally {
    loading.value = false;
  }
}

async function pickFile() {
  const picked = await open({
    title: "选择 .blend 文件",
    multiple: false,
    filters: [{name: "Blender 文件", extensions: ["blend"]}],
  });
  if (!picked || Array.isArray(picked)) return;
  await analyzePath(picked);
}

// 消费拖入窗口的 .blend
watch(
    () => [ui.page, ui.dropped_blends.length] as const,
    async ([page, n]) => {
      if (page === "blend" && n > 0) {
        const files = [...ui.dropped_blends];
        ui.dropped_blends = [];
        await analyzePath(files[0]);
        if (files.length > 1) {
          ui.notify("一次只分析一个文件，已取第一个", "info");
        }
      }
    },
    {immediate: true},
);

function percent(bytes: number): string {
  return (bytes / totalCatBytes.value * 100).toFixed(1) + "%";
}
</script>

<template>
  <div class="page-wrap">
    <div class="d-flex align-center mb-5">
      <div>
        <div class="page-title">文件分析</div>
        <div class="page-subtitle">看看 .blend 文件的体积都被什么占了</div>
      </div>
      <v-spacer/>
      <v-btn color="primary" variant="flat" prepend-icon="mdi-file-search-outline"
             :loading="loading" @click="pickFile">
        选择 .blend 文件
      </v-btn>
    </div>

    <div v-if="loading" class="d-flex align-center justify-center py-16">
      <v-progress-circular indeterminate color="primary" size="42"/>
      <span class="ml-4 dim">正在解析文件…</span>
    </div>

    <div v-else-if="!result" class="empty-state">
      <v-icon icon="mdi-chart-donut" size="52"/>
      <div class="text-h6">把 .blend 文件拖进窗口</div>
      <div class="text-body-2">支持未压缩 / gzip / zstd 压缩文件，兼容 Blender 5.0 新格式；分析在本地进行，不修改原文件</div>
    </div>

    <template v-else>
      <!-- 文件信息 -->
      <v-card class="card-soft mb-4">
        <v-card-text class="d-flex" style="gap: 18px">
          <div v-if="thumbUrl" class="thumb-box">
            <img :src="thumbUrl" alt="缩略图"/>
          </div>
          <div class="flex-grow-1" style="min-width: 0">
            <div class="d-flex align-center" style="gap: 10px">
              <span class="text-h6 font-weight-bold">{{ fileName }}</span>
              <v-chip size="x-small" variant="tonal" label>Blender {{ result.blender_version }}</v-chip>
              <v-chip size="x-small" variant="tonal" label
                      :color="result.compression === '无压缩' ? 'warning' : 'success'">
                {{ result.compression }}
              </v-chip>
              <v-chip size="x-small" variant="tonal" label>{{ result.header_kind }}</v-chip>
            </div>
            <div class="dim text-caption mono mt-1 text-truncate">{{ result.file }}</div>
            <div class="d-flex mt-3" style="gap: 32px">
              <div>
                <div class="text-h6 font-weight-bold">{{ formatBytes(result.file_size) }}</div>
                <div class="dim text-caption">磁盘占用</div>
              </div>
              <div>
                <div class="text-h6 font-weight-bold">{{ formatBytes(result.uncompressed_size) }}</div>
                <div class="dim text-caption">解压后数据量</div>
              </div>
              <div v-if="result.compression !== '无压缩'">
                <div class="text-h6 font-weight-bold">
                  {{ (result.file_size / Math.max(1, result.uncompressed_size) * 100).toFixed(0) }}%
                </div>
                <div class="dim text-caption">压缩率</div>
              </div>
              <div v-if="result.scenes.length > 0">
                <div class="text-h6 font-weight-bold mono">
                  {{ result.scenes[0].start }}-{{ result.scenes[0].end }}
                </div>
                <div class="dim text-caption">帧范围（{{ result.scenes[0].scene }}）</div>
              </div>
            </div>
          </div>
        </v-card-text>
      </v-card>

      <v-alert v-for="(w, i) in result.warnings" :key="i" type="warning" variant="tonal"
               density="compact" class="mb-3">
        {{ w }}
      </v-alert>

      <!-- 分类占比 -->
      <v-card class="card-soft mb-4">
        <v-card-title class="text-subtitle-1">体积构成</v-card-title>
        <v-card-text>
          <div class="stack-bar">
            <v-tooltip v-for="c in chartCats" :key="c.code"
                       :text="`${c.label}：${formatBytes(c.bytes)}（${percent(c.bytes)}，${c.count} 个）`">
              <template v-slot:activator="{ props }">
                <div v-bind="props" class="stack-seg"
                     :style="{width: (c.bytes / totalCatBytes * 100) + '%', background: c.color}"/>
              </template>
            </v-tooltip>
          </div>
          <div class="d-flex flex-wrap mt-3" style="gap: 6px 18px">
            <div v-for="c in chartCats" :key="c.code" class="d-flex align-center text-body-2" style="gap: 6px">
              <span class="legend-dot" :style="{background: c.color}"/>
              <span>{{ c.label }}</span>
              <span class="dim">{{ formatBytes(c.bytes) }} · {{ percent(c.bytes) }}</span>
            </div>
          </div>
        </v-card-text>
      </v-card>

      <!-- 最大数据块 -->
      <v-card class="card-soft mb-4">
        <v-card-title class="text-subtitle-1">最大的数据块</v-card-title>
        <v-table density="compact" class="bg-transparent">
          <thead>
          <tr>
            <th style="width: 42%">名称</th>
            <th>类别</th>
            <th class="text-right">体积</th>
            <th class="text-right">占比</th>
            <th class="text-right">附属块数</th>
          </tr>
          </thead>
          <tbody>
          <tr v-for="(b, i) in shownBlocks" :key="i">
            <td class="mono text-body-2 text-truncate" style="max-width: 380px">{{ b.name }}</td>
            <td>
              <v-chip size="x-small" variant="tonal" label
                      :style="{color: categoryColor(b.code)}">
                {{ b.label }}
              </v-chip>
            </td>
            <td class="text-right mono">{{ formatBytes(b.bytes) }}</td>
            <td class="text-right dim">{{ percent(b.bytes) }}</td>
            <td class="text-right dim">{{ b.data_blocks }}</td>
          </tr>
          </tbody>
        </v-table>
        <v-card-actions v-if="(result.top_blocks.length) > 15">
          <v-btn size="small" variant="text" block @click="showAllBlocks = !showAllBlocks">
            {{ showAllBlocks ? '收起' : `显示全部 ${result.top_blocks.length} 项` }}
          </v-btn>
        </v-card-actions>
      </v-card>

      <!-- 建议 -->
      <v-card class="card-soft">
        <v-card-title class="text-subtitle-1 d-flex align-center">
          优化建议
          <v-spacer/>
          <v-btn size="small" variant="tonal" color="primary" prepend-icon="mdi-auto-fix"
                 :loading="purging" @click="openPurgeDialog">
            清理孤立数据
          </v-btn>
        </v-card-title>
        <v-card-text class="pt-0">
          <div v-for="(s, i) in suggestions" :key="i" class="d-flex text-body-2 mb-2" style="gap: 8px">
            <v-icon icon="mdi-lightbulb-on-outline" size="16" color="primary" class="mt-1"/>
            <span>{{ s }}</span>
          </div>
        </v-card-text>
      </v-card>
    </template>

    <!-- 孤立数据清理确认 -->
    <v-dialog v-model="purgeDialog" max-width="500">
      <v-card class="card-soft">
        <v-card-title class="text-subtitle-1">清理孤立数据</v-card-title>
        <v-card-text class="text-body-2">
          <p class="mb-3">
            用 Blender 后台递归清除没有任何使用者的数据块（相当于「清理 → 递归清除孤立数据」），然后保存文件。
            <b>原文件会先自动备份为 .bak</b>。
          </p>
          <v-select v-model="purgeVersion" :items="configured" label="使用的 Blender 版本"
                    density="compact" hide-details/>
          <v-alert v-if="purgeVersionMismatch" type="warning" variant="tonal" density="compact" class="mt-3">
            文件保存于 Blender {{ result?.blender_version }}，用 {{ purgeVersion }} 保存会把文件升级到新版本格式，
            旧版本 Blender 可能无法打开。
          </v-alert>
        </v-card-text>
        <v-card-actions>
          <v-spacer/>
          <v-btn variant="text" @click="purgeDialog = false">取消</v-btn>
          <v-btn color="primary" variant="tonal" @click="doPurge">备份并清理</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.thumb-box {
  width: 128px;
  height: 128px;
  border-radius: 10px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.3);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.thumb-box img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.stack-bar {
  display: flex;
  height: 26px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.05);
}

.stack-seg {
  height: 100%;
  min-width: 2px;
  transition: filter .15s;
}

.stack-seg:hover {
  filter: brightness(1.25);
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 3px;
  display: inline-block;
}
</style>
