mod blend;
mod detect;
mod link;
mod maintenance;
mod procutil;
mod render;
mod startup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            link::link_dir,
            link::unlink_dir,
            link::remove_real_dir,
            link::is_symbolic_link,
            link::read_link_target,
            link::scan_addon_paths,
            link::check_link_status,
            link::read_addon_meta,
            maintenance::scan_cleanup,
            maintenance::run_cleanup,
            maintenance::purge_orphans,
            // WIP：前端尚未接线，先注册以免 dead_code
            maintenance::migrate_config,
            maintenance::check_blend_files,
            maintenance::unpack_blend,
            detect::detect_config_versions,
            detect::detect_blender_executables,
            detect::probe_blender_exe,
            detect::open_in_explorer,
            detect::launch_blender,
            blend::analyze_blend,
            blend::blend_meta,
            startup::startup_analyze,
            startup::startup_cancel,
            render::render_run,
            render::render_cancel,
            // WIP：并行渲染前端尚未接线
            render::render_cancel_all,
            render::schedule_shutdown,
            render::abort_shutdown
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
