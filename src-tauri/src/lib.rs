extern crate alloc;

use alloc::string::{String, ToString};
use fs::create_dir_all;
use std::fs;
use std::fs::{remove_dir_all, symlink_metadata};
use std::path::Path;
use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//mklink /J "C:\Users\Use\AppData\Roaming\Blender Foundation\Blender\4.2\extensions\user_default\aaa" "D:\Blender Addon\simple_deform_helper"
#[tauri::command]
fn link_dir(from: &str, to: &str) {
    let args = ["mklink", "/J", &to.to_string(), &from.to_string()];
    let path = Path::new(to).parent().unwrap();
    println!("link_dir from:{} to:{}", from, to);
    if path.exists() {
        println!("Path exists");
    } else {
        println!("Path does not exist");
        create_dir_all(path).expect("TODO: panic message");
    }

    let output = Command::new("cmd")
        .arg("/c")
        .args(&args)
        .output()
        .expect("Failed to execute command");

    println!("MkLink {}", String::from_utf8_lossy(&output.stdout));
    if output.status.success() {
        println!("Link created successfully {:?}", args);
    } else {
        eprintln!(
            "Failed to create link: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
#[tauri::command]
fn unlink_dir(ud: &str) -> Result<bool, String> {
    match remove_dir_all(ud) {
        Ok(_) => {
            if Path::new(ud).exists() {
                Err(String::from("Failed to delete directory"))
            } else {
                Ok(true)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn is_symbolic_link(path: &str) -> Result<bool, String> {
    match symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            link_dir,
            unlink_dir,
            is_symbolic_link
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
