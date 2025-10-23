import {appDataDir, dirname, join} from "@tauri-apps/api/path";

function addonIsExtension(version: string): boolean {
    /**
     * 5.1 -> 51
     * 4.1 > 41
     */
    const vn = parseInt(version.replace(".", ""))
    return vn > 41;
}

async function getAddonLinkFolder(version: string, is_extension: boolean) {
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

export {
    addonIsExtension,
    getAddonLinkFolder,
}