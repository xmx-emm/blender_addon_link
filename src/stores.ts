import {defineStore} from 'pinia';
import {AddonItem} from "./data";

const useBlenderAddonStore = defineStore("blender_addon", {
        state: () => ({
            blender_version_list: ['4.5', '5.0', '5.1'],
            addon_list: <AddonItem[]>[],
        }),
        actions: {
            restore_blender_version() {
                this.blender_version_list = ['4.5', '5.0', '5.1']
            },
            add_blender_version(version: string) {
                if (!this.blender_version_list.includes(version)) {
                    this.blender_version_list.push(version)
                }
            },
            remove_blender_version(version: string) {
                console.log('remove_blender_version', version)
                if (this.blender_version_list.includes(version)) {
                    this.blender_version_list = this.blender_version_list.filter((v) => v !== version)
                }
                console.log('this.blender_version_list', this.blender_version_list)
            },
            add_addon(addon: AddonItem) {
                if (!this.addon_list.includes(addon)) {
                    this.addon_list.push(addon)
                }
            },
            remove_addon(addon: AddonItem) {
                console.log('remove_addon', addon)
                if (this.addon_list.includes(addon)) {
                    this.addon_list = this.addon_list.filter((v) => v !== addon)
                }
            }
        },
        persist: true
    },
);
export default useBlenderAddonStore
