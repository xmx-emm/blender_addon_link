import {defineStore} from 'pinia';
import {invoke} from "@tauri-apps/api/core";
import {isPermissionGranted, requestPermission, sendNotification} from "@tauri-apps/plugin-notification";
import {AddonItem, RenderJob, RenderLogEvent, RenderOutcome, RenderProgressEvent} from "./data";
import {compareVersion, formatSeconds} from "@/utils/addon.ts";

/** 系统托盘通知（权限拿不到就静默跳过） */
async function notifySystem(title: string, body: string) {
    try {
        let granted = await isPermissionGranted();
        if (!granted) {
            granted = (await requestPermission()) === 'granted';
        }
        if (granted) {
            sendNotification({title, body});
        }
    } catch {
        // 通知失败不影响主流程
    }
}

export const DEFAULT_VERSIONS = ['4.2', '4.5', '5.0', '5.1', '5.2'];

const useBlenderAddonStore = defineStore("blender_addon", {
    state: () => ({
        blender_version_list: [...DEFAULT_VERSIONS],
        addon_list: <AddonItem[]>[],
        // 版本 -> blender.exe 路径
        exe_map: <Record<string, string>>{},
    }),
    getters: {
        sorted_versions(state): string[] {
            return [...state.blender_version_list].sort(compareVersion);
        },
    },
    actions: {
        restore_blender_version() {
            this.blender_version_list = [...DEFAULT_VERSIONS];
        },
        add_blender_version(version: string) {
            if (!this.blender_version_list.includes(version)) {
                this.blender_version_list.push(version);
            }
        },
        remove_blender_version(version: string) {
            this.blender_version_list = this.blender_version_list.filter((v) => v !== version);
        },
        add_addon(addon: AddonItem): boolean {
            if (this.addon_list.some((f) => f.addon_path === addon.addon_path)) {
                return false;
            }
            this.addon_list.push(addon);
            return true;
        },
        clear_addon() {
            this.addon_list = [];
        },
        remove_addon(addon: AddonItem) {
            this.addon_list = this.addon_list.filter((v) => v.addon_path !== addon.addon_path);
        },
        set_exe(version: string, path: string) {
            if (path) {
                this.exe_map[version] = path;
            } else {
                delete this.exe_map[version];
            }
        },
    },
    persist: true,
});
export default useBlenderAddonStore;

export const useUiStore = defineStore("ui", {
    state: () => ({
        page: 'link',
        snackbar: false,
        snackbar_text: '',
        snackbar_color: 'success',
        // 拖入窗口的 .blend 文件，由当前页面消费
        dropped_blends: <string[]>[],
        dragging: false,
    }),
    actions: {
        notify(text: string, color: 'success' | 'error' | 'warning' | 'info' = 'success') {
            this.snackbar_text = text;
            this.snackbar_color = color;
            this.snackbar = true;
        },
        ok(text: string) {
            this.notify(text, 'success');
        },
        error(text: string) {
            this.notify(text, 'error');
        },
    },
});

export const useRenderStore = defineStore("render_queue", {
    state: () => ({
        jobs: <RenderJob[]>[],
        shutdown_after: false,
        // 运行态（不持久化）
        queue_running: false,
        stop_requested: false,
        active_job_id: '',
        log_lines: <string[]>[],
        shutdown_scheduled: false,
    }),
    getters: {
        pending_count(state): number {
            return state.jobs.filter(j => j.status === 'pending').length;
        },
        active_job(state): RenderJob | undefined {
            return state.jobs.find(j => j.id === state.active_job_id);
        },
    },
    actions: {
        // ---- 队列执行（放在 store 里，切换页面不中断）----
        async start_queue() {
            if (this.queue_running) return;
            this.queue_running = true;
            this.stop_requested = false;
            const ui = useUiStore();
            let ran = 0;
            let ok = 0;
            let failed = 0;
            try {
                while (!this.stop_requested) {
                    const job = this.jobs.find(j => j.status === 'pending');
                    if (!job) break;
                    const exe = useBlenderAddonStore().exe_map[job.version];
                    if (!exe) {
                        job.status = 'failed';
                        job.error = `未配置 Blender ${job.version} 的 blender.exe（请到设置页配置）`;
                        ui.error(job.error);
                        continue;
                    }
                    job.status = 'running';
                    this.active_job_id = job.id;
                    this.log_lines = [];
                    try {
                        const outcome = await invoke<RenderOutcome>('render_run', {
                            spec: {
                                id: job.id,
                                exe,
                                blend: job.blend,
                                mode: job.mode,
                                frame_start: job.mode === 'range' ? job.frame_start : null,
                                frame_end: job.mode === 'range' ? job.frame_end : null,
                                frame: job.mode === 'frame' ? job.frame : null,
                                scene: job.scene || null,
                                engine: job.engine || null,
                                output: job.output || null,
                                extra_args: null,
                            },
                        });
                        job.saved_count = outcome.saved_count;
                        job.seconds = outcome.seconds;
                        ran++;
                        if (outcome.cancelled) {
                            job.status = 'cancelled';
                            ui.notify(`已取消：${job.name}`, 'warning');
                        } else if (outcome.success) {
                            job.status = 'done';
                            ok++;
                            ui.ok(`渲染完成：${job.name}（${outcome.saved_count} 帧，${formatSeconds(outcome.seconds)}）`);
                        } else {
                            job.status = 'failed';
                            failed++;
                            job.error = `Blender 退出码 ${outcome.code}`;
                            ui.error(`渲染失败：${job.name}（退出码 ${outcome.code}）`);
                        }
                    } catch (e) {
                        ran++;
                        failed++;
                        job.status = 'failed';
                        job.error = String(e);
                        ui.error(`渲染失败：${job.name} — ${e}`);
                    }
                    this.active_job_id = '';
                }
            } finally {
                this.queue_running = false;
                this.active_job_id = '';
            }
            if (ran > 0) {
                notifySystem(
                    '渲染队列结束',
                    failed > 0 ? `${ok} 个成功，${failed} 个失败` : `全部 ${ok} 个任务完成`,
                );
            }
            const unfinished = this.jobs.some(j => j.status === 'pending' || j.status === 'running');
            if (!unfinished && this.jobs.length > 0 && this.shutdown_after && !this.stop_requested) {
                try {
                    await invoke('schedule_shutdown', {seconds: 60});
                    this.shutdown_scheduled = true;
                    useUiStore().notify('队列完成，60 秒后关机（可在渲染页撤销）', 'warning');
                } catch (e) {
                    useUiStore().error(`计划关机失败：${e}`);
                }
            }
        },
        stop_after_current() {
            this.stop_requested = true;
        },
        async cancel_current() {
            try {
                await invoke('render_cancel');
            } catch (e) {
                useUiStore().error(String(e));
            }
        },
        async abort_shutdown() {
            try {
                await invoke('abort_shutdown');
                this.shutdown_scheduled = false;
                useUiStore().ok('已撤销关机');
            } catch (e) {
                useUiStore().error(`撤销关机失败：${e}`);
            }
        },
        on_progress(ev: RenderProgressEvent) {
            const j = this.jobs.find(x => x.id === ev.job_id);
            if (!j) return;
            if (ev.frame != null) j.current_frame = ev.frame;
            if (ev.sample != null) {
                j.sample = ev.sample;
                j.sample_total = ev.sample_total;
            }
            j.saved_count = ev.saved_count;
            j.seconds = ev.elapsed_seconds;
        },
        on_log(ev: RenderLogEvent) {
            if (ev.job_id === this.active_job_id) {
                this.push_log(ev.line);
            }
        },
        add_job(job: RenderJob) {
            this.jobs.push(job);
        },
        remove_job(id: string) {
            this.jobs = this.jobs.filter(j => j.id !== id);
        },
        move_job(id: string, dir: -1 | 1) {
            const i = this.jobs.findIndex(j => j.id === id);
            const t = i + dir;
            if (i < 0 || t < 0 || t >= this.jobs.length) return;
            const [j] = this.jobs.splice(i, 1);
            this.jobs.splice(t, 0, j);
        },
        clear_finished() {
            this.jobs = this.jobs.filter(j => j.status === 'pending' || j.status === 'running');
        },
        reset_job(id: string) {
            const j = this.jobs.find(x => x.id === id);
            if (j && j.status !== 'running') {
                j.status = 'pending';
                j.current_frame = null;
                j.sample = null;
                j.sample_total = null;
                j.saved_count = 0;
                j.seconds = 0;
                j.error = '';
            }
        },
        push_log(line: string) {
            this.log_lines.push(line);
            if (this.log_lines.length > 500) {
                this.log_lines.splice(0, 250);
            }
        },
        // 应用重启后，把中断的"渲染中"任务恢复为等待
        recover() {
            this.queue_running = false;
            this.active_job_id = '';
            for (const j of this.jobs) {
                if (j.status === 'running') {
                    j.status = 'pending';
                }
            }
        },
    },
    persist: {
        pick: ['jobs', 'shutdown_after'],
    },
});
