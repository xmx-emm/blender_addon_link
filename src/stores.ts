import {defineStore} from "pinia";


export const useBlenderAddonStore = defineStore("useBlenderAddonStore", {
    state: () => ({
        blender_version_list: ['4.5', '5.0', '5.1'],
        addon_path_list: <string[]>[],
    }),
    actions: {
        add_blender_version(version: string) {
            if (!this.blender_version_list.includes(version)) {
                this.blender_version_list.push(version)
            }
        },
        remove_blender_version(version: string) {
            if (this.blender_version_list.includes(version)) {
                this.blender_version_list = this.blender_version_list.filter((f) => f != version)
            }
        },
        add_addon_path(addon_path: string) {
            if (!this.addon_path_list.includes(addon_path)) {
                this.addon_path_list.push(addon_path)
            }
        },
        remove_addon_path(addon_path: string) {
            if (this.addon_path_list.includes(addon_path)) {
                this.addon_path_list = this.addon_path_list.filter((f) => f != addon_path)
            }
        }
    },
    persist: true,
});