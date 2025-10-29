import {invoke} from "@tauri-apps/api/core";
import {remove} from "@tauri-apps/plugin-fs";

function linkAddon(from_path: string, to_path: string) {
    const args = {to: to_path, from: from_path}
    return invoke("link_dir", args)
}

function unlinkAddon(addon_path: string) {
    return remove(addon_path)
}

export {
    linkAddon,
    unlinkAddon
}