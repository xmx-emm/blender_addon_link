export interface AddonItem {
    addon_path: string,
    is_extension: boolean,//blender_manifest.toml 是新的扩展
}

export interface AddonLinkItem {
    blender_version: string,
    install_folder: string,
    is_exists: boolean,
    is_symbolic_link: boolean,//是link的文件
}