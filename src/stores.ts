import {defineStore} from 'pinia';
import {AddonItem} from "./data";

export const useStore = defineStore('main', {
    state: () => {
        return {
            someState: 'hello pinia',
        }
    },
    persist: true,
})

export const useBlenderAddonStore = defineStore("blender_addon", {
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
                const index = this.blender_version_list.indexOf(version);
                console.log('remove_blender_version', version, index)
                if (index > -1) {
                    this.blender_version_list.splice(index, 1);
                }
            },
            add_addon(addon: AddonItem) {
                if (!this.addon_list.includes(addon)) {
                    this.addon_list.push(addon)
                }
            },
            remove_addon(addon: AddonItem) {
                const index = this.addon_list.indexOf(addon);
                if (index > -1) {
                    this.addon_list.splice(index, 1);
                }
            }
        },
        persist: true
    },
);
