<script setup lang="ts">
import {computed, nextTick, onMounted, onUnmounted, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {open} from "@tauri-apps/plugin-dialog";
import useBlenderAddonStore, {useRenderStore, useUiStore} from "@/stores.ts";
import {BlendMeta, RenderJob, RenderLogEvent, RenderMode, RenderProgressEvent} from "@/data.ts";
import {formatSeconds, thumbToDataUrl} from "@/utils/addon.ts";

const store = useBlenderAddonStore();
const render = useRenderStore();
const ui = useUiStore();

const configured = computed(() => store.sorted_versions.filter(v => !!store.exe_map[v]));
const defaultVersion = computed(() => configured.value[configured.value.length - 1] ?? "");

// ---- 添加/编辑任务对话框 ----
const dialog = ref(false);
const dlgEditId = ref<string | null>(null);
const dlgFile = ref("");
const dlgMeta = ref<BlendMeta | null>(null);
const dlgMetaError = ref("");
const dlgThumbUrl = ref<string | null>(null);
const dlgVersion = ref("");
const dlgMode = ref<RenderMode>("animation");
const dlgStart = ref<number | string | null>(null);
const dlgEnd = ref<number | string | null>(null);
const dlgFrame = ref<number | string | null>(1);
const dlgScene = ref("");
const dlgEngine = ref("");
const dlgOutput = ref("");

const dlgFileName = computed(() => dlgFile.value.split(/[\\/]/).pop() ?? "");
const dlgThumb = computed(() => {
  if (dlgThumbUrl.value) return dlgThumbUrl.value;
  const t = dlgMeta.value?.thumbnail;
  return t ? thumbToDataUrl(t.width, t.height, t.rgba_base64) : null;
});

async function openDialogWith(path: string) {
  dlgEditId.value = null;
  dlgFile.value = path;
  dlgMeta.value = null;
  dlgMetaError.value = "";
  dlgThumbUrl.value = null;
  dlgVersion.value = defaultVersion.value;
  dlgMode.value = "animation";
  dlgStart.value = null;
  dlgEnd.value = null;
  dlgFrame.value = 1;
  dlgScene.value = "";
  dlgEngine.value = "";
  dlgOutput.value = "";
  dialog.value = true;
  try {
    const meta = await invoke<BlendMeta>("blend_meta", {path});
    dlgMeta.value = meta;
    if (meta.scenes.length > 0) {
      dlgStart.value = meta.scenes[0].start;
      dlgEnd.value = meta.scenes[0].end;
      dlgFrame.value = meta.scenes[0].start;
    }
  } catch (e) {
    dlgMetaError.value = String(e);
  }
}

async function pickBlend() {
  const picked = await open({
    title: "选择要渲染的 .blend 文件",
    multiple: false,
    filters: [{name: "Blender 文件", extensions: ["blend"]}],
  });
  if (!picked || Array.isArray(picked)) return;
  await openDialogWith(picked);
}

function makeJob(path: string, meta: BlendMeta | null): RenderJob {
  const scene0 = meta?.scenes?.[0];
  const t = meta?.thumbnail;
  return {
    id: crypto.randomUUID(),
    blend: path,
    name: path.split(/[\\/]/).pop() ?? path,
    version: defaultVersion.value,
    mode: "animation",
    frame_start: null,
    frame_end: null,
    frame: null,
    scene: "",
    engine: "",
    output: "",
    status: "pending",
    current_frame: null,
    sample: null,
    sample_total: null,
    saved_count: 0,
    seconds: 0,
    error: "",
    meta_start: scene0?.start ?? null,
    meta_end: scene0?.end ?? null,
    meta_scene: scene0?.scene ?? "",
    thumb_data_url: t ? thumbToDataUrl(t.width, t.height, t.rgba_base64) : null,
    blender_version: meta?.blender_version ?? "",
  };
}

function toInt(v: unknown): number | null {
  const n = typeof v === "string" ? parseInt(v, 10) : typeof v === "number" ? v : NaN;
  return Number.isFinite(n) ? Math.trunc(n) : null;
}

function editJob(j: RenderJob) {
  dlgEditId.value = j.id;
  dlgFile.value = j.blend;
  dlgMetaError.value = "";
  dlgThumbUrl.value = j.thumb_data_url;
  dlgMeta.value = {
    blender_version: j.blender_version,
    compression: "",
    file_size: 0,
    scenes: j.meta_start != null && j.meta_end != null
        ? [{scene: j.meta_scene, start: j.meta_start, end: j.meta_end}]
        : [],
    thumbnail: null,
  };
  dlgVersion.value = j.version;
  dlgMode.value = j.mode;
  dlgStart.value = j.frame_start;
  dlgEnd.value = j.frame_end;
  dlgFrame.value = j.frame;
  dlgScene.value = j.scene;
  dlgEngine.value = j.engine;
  dlgOutput.value = j.output;
  dialog.value = true;
}

function confirmAdd() {
  if (!dlgFile.value || !dlgVersion.value) return;
  const start = toInt(dlgStart.value);
  const end = toInt(dlgEnd.value);
  const frame = toInt(dlgFrame.value);
  if (dlgMode.value === "range" && (start == null || end == null)) {
    ui.notify("请填写帧区间", "warning");
    return;
  }
  if (dlgMode.value === "frame" && frame == null) {
    ui.notify("请填写帧号", "warning");
    return;
  }
  if (dlgEditId.value) {
    const j = render.jobs.find(x => x.id === dlgEditId.value);
    if (j && j.status !== "running") {
      j.version = dlgVersion.value;
      j.mode = dlgMode.value;
      j.frame_start = start;
      j.frame_end = end;
      j.frame = frame;
      j.scene = dlgScene.value.trim();
      j.engine = dlgEngine.value.trim();
      j.output = dlgOutput.value.trim();
      render.reset_job(j.id);
      j.status = "pending";
      ui.ok(`已更新任务：${j.name}`);
    }
    dialog.value = false;
    return;
  }
  const job = makeJob(dlgFile.value, dlgMeta.value);
  job.version = dlgVersion.value;
  job.mode = dlgMode.value;
  job.frame_start = start;
  job.frame_end = end;
  job.frame = frame;
  job.scene = dlgScene.value.trim();
  job.engine = dlgEngine.value.trim();
  job.output = dlgOutput.value.trim();
  render.add_job(job);
  dialog.value = false;
  ui.ok(`已加入队列：${job.name}`);
}

/** 中断的动画任务可以从最后渲染到的帧继续 */
function canResume(j: RenderJob): boolean {
  if (j.status !== "cancelled" && j.status !== "failed") return false;
  if (j.mode === "frame" || j.current_frame == null) return false;
  const r = frameRange(j);
  return r != null && j.current_frame >= r[0] && j.current_frame <= r[1];
}

function resumeJob(j: RenderJob) {
  const r = frameRange(j);
  if (!r || j.current_frame == null) return;
  const from = j.current_frame;
  j.mode = "range";
  j.frame_start = from;
  j.frame_end = r[1];
  render.reset_job(j.id);
  ui.ok(`已重新排队：${j.name} 将从第 ${from} 帧继续`);
}

// 拖入 .blend：直接按默认设置入队
watch(
    () => [ui.page, ui.dropped_blends.length] as const,
    async ([page, n]) => {
      if (page !== "render" || n === 0) return;
      const files = [...ui.dropped_blends];
      ui.dropped_blends = [];
      if (configured.value.length === 0) {
        ui.notify("请先到设置页配置 blender.exe，再添加渲染任务", "warning");
        return;
      }
      for (const f of files) {
        let meta: BlendMeta | null = null;
        try {
          meta = await invoke<BlendMeta>("blend_meta", {path: f});
        } catch { /* 元信息读不到不影响入队 */
        }
        render.add_job(makeJob(f, meta));
      }
      ui.ok(`已添加 ${files.length} 个任务（动画模式，使用文件内设置），可删除后通过「添加任务」自定义`);
    },
);

// ---- 队列执行与进度 ----
function frameRange(j: RenderJob): [number, number] | null {
  if (j.mode === "range" && j.frame_start != null && j.frame_end != null) {
    return [j.frame_start, j.frame_end];
  }
  if (j.mode !== "frame" && j.meta_start != null && j.meta_end != null) {
    return [j.meta_start, j.meta_end];
  }
  return null;
}

function jobProgress(j: RenderJob): number | null {
  if (j.status === "done") return 100;
  if (j.status !== "running") return null;
  const sampleFrac = j.sample && j.sample_total ? Math.min(1, j.sample / j.sample_total) : 0;
  if (j.mode === "frame") {
    return j.sample_total ? sampleFrac * 100 : null;
  }
  const range = frameRange(j);
  if (!range) return null;
  const total = Math.max(1, range[1] - range[0] + 1);
  return Math.min(100, (j.saved_count + sampleFrac) / total * 100);
}

function modeText(j: RenderJob): string {
  if (j.mode === "frame") return `单帧 ${j.frame}`;
  if (j.mode === "range") return `帧 ${j.frame_start} - ${j.frame_end}`;
  const r = frameRange(j);
  return r ? `动画 ${r[0]} - ${r[1]}` : "动画（文件设置）";
}

const statusMap: Record<string, { text: string, color: string, icon: string }> = {
  pending: {text: "等待", color: "grey", icon: "mdi-clock-outline"},
  running: {text: "渲染中", color: "primary", icon: "mdi-play-circle-outline"},
  done: {text: "完成", color: "success", icon: "mdi-check-circle-outline"},
  failed: {text: "失败", color: "error", icon: "mdi-alert-circle-outline"},
  cancelled: {text: "已取消", color: "warning", icon: "mdi-cancel"},
};

// ---- 日志 ----
const showLog = ref(false);
const logBox = ref<HTMLElement | null>(null);
watch(() => render.log_lines.length, async () => {
  await nextTick();
  if (logBox.value) {
    logBox.value.scrollTop = logBox.value.scrollHeight;
  }
});

const unlisteners: UnlistenFn[] = [];
onMounted(async () => {
  unlisteners.push(await listen<RenderProgressEvent>("render-progress", e => render.on_progress(e.payload)));
  unlisteners.push(await listen<RenderLogEvent>("render-log", e => render.on_log(e.payload)));
});
onUnmounted(() => unlisteners.forEach(u => u()));
</script>

<template>
  <div class="page-wrap">
    <div class="d-flex align-center mb-5">
      <div>
        <div class="page-title">渲染队列</div>
        <div class="page-subtitle">批量后台渲染多个 .blend，睡觉时让电脑干活</div>
      </div>
      <v-spacer/>
      <v-btn color="primary" variant="flat" prepend-icon="mdi-plus" @click="pickBlend">添加任务</v-btn>
    </div>

    <!-- 控制条 -->
    <v-card class="card-soft mb-4">
      <v-card-text class="d-flex align-center flex-wrap" style="gap: 10px">
        <v-btn v-if="!render.queue_running" color="primary" variant="tonal" prepend-icon="mdi-play"
               :disabled="render.pending_count === 0" @click="render.start_queue()">
          开始队列（{{ render.pending_count }} 个等待）
        </v-btn>
        <template v-else>
          <v-btn v-if="!render.stop_requested" color="warning" variant="tonal" prepend-icon="mdi-pause"
                 @click="render.stop_after_current()">
            当前任务完成后停止
          </v-btn>
          <v-chip v-else color="warning" variant="tonal" prepend-icon="mdi-pause">当前任务完成后将停止</v-chip>
          <v-btn color="error" variant="tonal" prepend-icon="mdi-stop" @click="render.cancel_current()">
            取消当前任务
          </v-btn>
        </template>
        <v-btn variant="text" size="small" prepend-icon="mdi-broom"
               :disabled="render.jobs.every(j => j.status === 'pending' || j.status === 'running')"
               @click="render.clear_finished()">
          清除已结束
        </v-btn>
        <v-spacer/>
        <v-btn v-if="render.shutdown_scheduled" color="error" variant="flat" size="small"
               prepend-icon="mdi-power-plug-off" @click="render.abort_shutdown()">
          撤销关机！
        </v-btn>
        <v-switch
            v-model="render.shutdown_after"
            label="全部完成后关机"
            color="primary"
            density="compact"
            hide-details
        />
      </v-card-text>
    </v-card>

    <!-- 任务列表 -->
    <template v-if="render.jobs.length > 0">
      <v-card v-for="(j, idx) in render.jobs" :key="j.id" class="card-soft mb-3"
              :class="{'job-running': j.status === 'running'}">
        <v-card-text class="d-flex" style="gap: 14px">
          <div class="job-thumb">
            <img v-if="j.thumb_data_url" :src="j.thumb_data_url" alt=""/>
            <v-icon v-else icon="mdi-movie-outline" size="28" class="dim"/>
          </div>
          <div class="flex-grow-1" style="min-width: 0">
            <div class="d-flex align-center" style="gap: 8px">
              <span class="font-weight-bold text-body-1 text-truncate">{{ j.name }}</span>
              <v-chip size="x-small" :color="statusMap[j.status].color" variant="tonal" label
                      :prepend-icon="statusMap[j.status].icon">
                {{ statusMap[j.status].text }}
              </v-chip>
              <v-chip size="x-small" variant="tonal" label class="mono">Blender {{ j.version }}</v-chip>
              <span class="dim text-caption">{{ modeText(j) }}</span>
              <span v-if="j.scene" class="dim text-caption">场景 {{ j.scene }}</span>
              <span v-if="j.engine" class="dim text-caption">{{ j.engine }}</span>
            </div>
            <div class="dim text-caption mono text-truncate mt-1">{{ j.blend }}</div>

            <!-- 进度 -->
            <div v-if="j.status === 'running'" class="mt-2">
              <div class="d-flex align-center text-caption mb-1" style="gap: 12px">
                <span v-if="j.current_frame != null">帧 {{ j.current_frame }}</span>
                <span v-if="j.sample && j.sample_total">采样 {{ j.sample }}/{{ j.sample_total }}</span>
                <span v-if="j.saved_count">已保存 {{ j.saved_count }} 帧</span>
                <span class="dim">{{ formatSeconds(j.seconds) }}</span>
              </div>
              <v-progress-linear
                  :model-value="jobProgress(j) ?? 0"
                  :indeterminate="jobProgress(j) === null"
                  color="primary" rounded height="6"
              />
            </div>
            <div v-else-if="j.status === 'done'" class="text-caption text-success mt-1">
              {{ j.saved_count }} 帧 · 用时 {{ formatSeconds(j.seconds) }}
            </div>
            <div v-else-if="j.status === 'failed'" class="text-caption text-error mt-1">{{ j.error }}</div>
          </div>

          <div class="d-flex flex-column" style="gap: 2px">
            <v-btn icon="mdi-chevron-up" size="x-small" variant="text"
                   :disabled="idx === 0 || j.status === 'running'" @click="render.move_job(j.id, -1)"/>
            <v-btn icon="mdi-chevron-down" size="x-small" variant="text"
                   :disabled="idx === render.jobs.length - 1 || j.status === 'running'"
                   @click="render.move_job(j.id, 1)"/>
          </div>
          <div class="d-flex flex-column" style="gap: 2px">
            <v-btn v-if="j.status !== 'running'" icon="mdi-pencil-outline" size="x-small" variant="text"
                   title="编辑任务" @click="editJob(j)"/>
            <v-btn v-if="canResume(j)" icon="mdi-play-speed" size="x-small" variant="text" color="primary"
                   :title="`从第 ${j.current_frame} 帧继续`" @click="resumeJob(j)"/>
            <v-btn v-else-if="j.status !== 'running' && j.status !== 'pending'" icon="mdi-restart" size="x-small"
                   variant="text" title="重新排队（从头渲染）" @click="render.reset_job(j.id)"/>
            <v-btn icon="mdi-delete-outline" size="x-small" variant="text" title="移除"
                   :disabled="j.status === 'running'" @click="render.remove_job(j.id)"/>
          </div>
        </v-card-text>
      </v-card>

      <!-- 实时日志 -->
      <v-card class="card-soft">
        <v-card-title class="text-subtitle-2 d-flex align-center" style="cursor: pointer"
                      @click="showLog = !showLog">
          <v-icon :icon="showLog ? 'mdi-chevron-down' : 'mdi-chevron-right'" size="18" class="mr-1"/>
          实时日志
          <span v-if="render.active_job" class="dim text-caption ml-2">{{ render.active_job.name }}</span>
        </v-card-title>
        <v-expand-transition>
          <div v-if="showLog">
            <v-divider/>
            <div ref="logBox" class="log-box mono">
              <div v-for="(line, i) in render.log_lines" :key="i">{{ line }}</div>
              <div v-if="render.log_lines.length === 0" class="dim">（暂无输出）</div>
            </div>
          </div>
        </v-expand-transition>
      </v-card>
    </template>

    <div v-else class="empty-state">
      <v-icon icon="mdi-movie-open-play-outline" size="52"/>
      <div class="text-h6">队列是空的</div>
      <div class="text-body-2">把 .blend 文件拖进窗口快速入队，或点「添加任务」自定义帧区间、场景、引擎和输出路径</div>
    </div>

    <!-- 添加/编辑任务对话框 -->
    <v-dialog v-model="dialog" max-width="640">
      <v-card class="card-soft pa-2">
        <v-card-title class="text-subtitle-1">{{ dlgEditId ? '编辑渲染任务' : '添加渲染任务' }}</v-card-title>
        <v-card-text>
          <div class="d-flex mb-4" style="gap: 14px">
            <div class="job-thumb">
              <img v-if="dlgThumb" :src="dlgThumb" alt=""/>
              <v-icon v-else icon="mdi-movie-outline" size="28" class="dim"/>
            </div>
            <div style="min-width: 0">
              <div class="font-weight-bold">{{ dlgFileName }}</div>
              <div class="dim text-caption mono text-truncate">{{ dlgFile }}</div>
              <div v-if="dlgMeta" class="text-caption mt-1">
                <v-chip v-if="dlgMeta.blender_version" size="x-small" variant="tonal" label class="mr-1">
                  保存于 Blender {{ dlgMeta.blender_version }}
                </v-chip>
                <span v-if="dlgMeta.scenes.length" class="dim">
                  场景 {{ dlgMeta.scenes[0].scene }} · 帧 {{ dlgMeta.scenes[0].start }}-{{ dlgMeta.scenes[0].end }}
                </span>
              </div>
              <div v-else-if="dlgMetaError" class="text-caption text-warning mt-1">
                读取元信息失败：{{ dlgMetaError }}
              </div>
            </div>
          </div>

          <v-row dense>
            <v-col cols="6">
              <v-select v-model="dlgVersion" :items="configured" label="Blender 版本" density="compact"
                        hide-details/>
            </v-col>
            <v-col cols="6">
              <v-combobox v-model="dlgEngine" :items="['CYCLES', 'BLENDER_EEVEE_NEXT', 'BLENDER_WORKBENCH']"
                          label="引擎（留空 = 文件设置）" density="compact" hide-details clearable/>
            </v-col>
          </v-row>

          <v-radio-group v-model="dlgMode" inline hide-details class="my-2">
            <v-radio label="整段动画" value="animation"/>
            <v-radio label="帧区间" value="range"/>
            <v-radio label="单帧" value="frame"/>
          </v-radio-group>

          <v-row dense v-if="dlgMode === 'range'">
            <v-col cols="6">
              <v-text-field v-model="dlgStart" label="起始帧" type="number" density="compact" hide-details/>
            </v-col>
            <v-col cols="6">
              <v-text-field v-model="dlgEnd" label="结束帧" type="number" density="compact" hide-details/>
            </v-col>
          </v-row>
          <v-row dense v-if="dlgMode === 'frame'">
            <v-col cols="6">
              <v-text-field v-model="dlgFrame" label="帧号" type="number" density="compact" hide-details/>
            </v-col>
          </v-row>
          <div v-if="dlgMode === 'animation'" class="dim text-caption mb-2">
            使用文件内的帧范围与输出设置
          </div>

          <v-row dense>
            <v-col cols="6">
              <v-text-field v-model="dlgScene" label="场景（留空 = 文件设置）" density="compact" hide-details
                            :placeholder="dlgMeta?.scenes?.[0]?.scene ?? ''"/>
            </v-col>
            <v-col cols="6">
              <v-text-field v-model="dlgOutput" label="输出路径覆盖（可选）" density="compact"
                            placeholder="如 D:\render\out_####"
                            hint="#### 为帧号占位；4.5+ 还支持 {blend_name} 等模板" persistent-hint/>
            </v-col>
          </v-row>
        </v-card-text>
        <v-card-actions>
          <v-spacer/>
          <v-btn variant="text" @click="dialog = false">取消</v-btn>
          <v-btn color="primary" variant="flat" :disabled="!dlgVersion" @click="confirmAdd">
            {{ dlgEditId ? '保存修改' : '加入队列' }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.job-thumb {
  width: 72px;
  height: 72px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.3);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.job-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.job-running {
  border-color: rgba(232, 125, 13, 0.45);
}

.log-box {
  height: 260px;
  overflow-y: auto;
  padding: 10px 14px;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
  background: rgba(0, 0, 0, 0.25);
}
</style>
