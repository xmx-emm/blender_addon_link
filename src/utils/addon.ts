import {appDataDir, dirname, join} from "@tauri-apps/api/path";
import {lstat, readDir} from "@tauri-apps/plugin-fs";
import useBlenderAddonStore from "../stores";

const blenderAddonStore = useBlenderAddonStore()

export function addonIsExtension(version: string): boolean {
    /**
     * 5.1 -> 51
     * 4.1 > 41
     */
    const vn = parseInt(version.replace(".", ""))
    return vn > 41;
}

export async function getAddonLinkFolder(version: string, is_extension: boolean) {
    /**
     * 通过版本获取插件文件夹路径
     * 4.1没有扩展文件夹,4.2 >= 才有扩展
     * blender版本目前只有两位数 2024-10-28
     * C:\Users\Name\AppData\Roaming\Blender Foundation\Blender\4.4\extensions\user_default
     * C:\Users\Name\AppData\Roaming\Blender Foundation\Blender\4.4\script\addons
     */
    const extension = addonIsExtension(version)
    const pathJoin = [
        await dirname(await dirname(await appDataDir())),
        // 'AppData',
        'Roaming',
        'Blender Foundation',
        'Blender',
        version
    ]

    if (is_extension && extension) {
        pathJoin.push('extensions', 'user_default')
    } else {
        pathJoin.push('scripts', 'addons')
    }
    return await join(...pathJoin)
}


export function checkAddon(addon_path: string, pathDeep = 0) {
    /**
     *  查找文件 __init__.py,blender_manifest.toml
     *  如果有 blender_manifest.toml 则是新版本的插件
     */

    readDir(addon_path).then(async (entry) => {
        for (const file of entry) {
            if (file.isFile) {
                if (file.name === "blender_manifest.toml") {
                    blenderAddonStore.addon_list.push({is_extension: true, addon_path: addon_path,})
                    return
                } else if (file.name === "__init__.py") {
                    blenderAddonStore.addon_list.push({is_extension: false, addon_path: addon_path,})
                    return
                }
            }
        }
        if (pathDeep < 1) {
            // 既不是插件，也不是插件文件夹
            // 可能是嵌套文件夹
            for (const file of entry) {
                if (file.isDirectory) {
                    const joinPath = await join(addon_path, file.name)
                    checkAddon(joinPath, pathDeep + 1)
                }
            }
        }
    })
}

export function findAddon(addon_path: string) {
    // 1.检查是否为插件
    // 2.检查是否为插件文件夹
    lstat(addon_path).then(fi => {
        if (fi.isDirectory) {
            checkAddon(addon_path)
        }
    })
}
