export interface AddonItem {
    addon_path: string,
    is_extension: boolean, // 有 blender_manifest.toml 即为新版扩展
    is_expand: boolean,
}

export interface AddonLinkItem {
    blender_version: string,
    install_folder: string,
    is_exists: boolean,
    is_symbolic_link: boolean,
    /** 链接目标是否等于当前插件源码路径 */
    matches_source?: boolean,
    link_target?: string, // 链接指向的真实路径
    /** 另一安装位置（addons ↔ extensions）是否也存在 */
    dual_install?: boolean,
    supported: boolean,   // 该版本是否支持此插件形态
}

export interface BlenderExe {
    version: string,
    path: string,
    source: string,
}

export interface AddonMeta {
    name: string,
    version: string,
    blender_min: string,
}

// ---- 磁盘清理 ----
export interface CleanupTarget {
    id: string,
    label: string,
    path: string,
    bytes: number,
    files: number,
}

export interface CleanupResult {
    freed: number,
    deleted: number,
    errors: string[],
}

export interface PurgeResult {
    removed: number,
    old_size: number,
    new_size: number,
    backup: string,
}

// ---- 启动分析 ----
export interface AddonTiming {
    module: string,
    display_name: string,
    seconds: number,
    ok: boolean,
    error: string,
}

export interface StartupResult {
    warmup_seconds: number,
    normal_seconds: number[],
    factory_seconds: number[],
    addons: AddonTiming[],
    notes: string[],
}

export interface StartupProgress {
    step: number,
    total: number,
    message: string,
}

// ---- 文件分析 ----
export interface CategoryStat {
    code: string,
    label: string,
    count: number,
    bytes: number,
}

export interface BlockStat {
    name: string,
    code: string,
    label: string,
    bytes: number,
    data_blocks: number,
}

export interface RendInfo {
    scene: string,
    start: number,
    end: number,
}

export interface Thumb {
    width: number,
    height: number,
    rgba_base64: string,
}

export interface BlendAnalysis {
    file: string,
    file_size: number,
    compression: string,
    blender_version: string,
    pointer_size: number,
    header_kind: string,
    uncompressed_size: number,
    categories: CategoryStat[],
    top_blocks: BlockStat[],
    scenes: RendInfo[],
    thumbnail: Thumb | null,
    warnings: string[],
}

export interface BlendMeta {
    blender_version: string,
    compression: string,
    file_size: number,
    scenes: RendInfo[],
    thumbnail: Thumb | null,
}

// ---- 渲染队列 ----
export type RenderMode = 'animation' | 'range' | 'frame'
export type JobStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled'

export interface RenderJob {
    id: string,
    blend: string,
    name: string,
    version: string,
    mode: RenderMode,
    frame_start: number | null,
    frame_end: number | null,
    frame: number | null,
    scene: string,
    engine: string,
    output: string,
    status: JobStatus,
    current_frame: number | null,
    sample: number | null,
    sample_total: number | null,
    saved_count: number,
    seconds: number,
    error: string,
    // 从 .blend 读到的元信息（用于进度估算与展示）
    meta_start: number | null,
    meta_end: number | null,
    meta_scene: string,
    thumb_data_url: string | null,
    blender_version: string,
}

export interface RenderOutcome {
    code: number,
    success: boolean,
    cancelled: boolean,
    saved_count: number,
    seconds: number,
    tail: string[],
}

export interface RenderProgressEvent {
    job_id: string,
    frame: number | null,
    mem_mb: number | null,
    sample: number | null,
    sample_total: number | null,
    saved: string | null,
    saved_count: number,
    elapsed_seconds: number,
}

export interface RenderLogEvent {
    job_id: string,
    line: string,
}
