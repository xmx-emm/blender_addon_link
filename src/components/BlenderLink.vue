<script setup lang="ts">
// 拖入文件夹设置文件夹
// 读取那些Blender版本被Link了的,或者是已经有了的
// 功能:
// 1.检查是否为Link文件夹，取消Link
// 2.添加Link，如果不是Link的可以删除重新Link
// 4.列出可添加版本

import {Event, listen} from "@tauri-apps/api/event";
import {lstat, readDir, WatchEvent} from "@tauri-apps/plugin-fs";
import {join} from "@tauri-apps/api/path";
import useBlenderAddonStore from "@/stores.ts";
import LinkAddonList from "@/components/LinkAddonList.vue";


const blenderAddonStore = useBlenderAddonStore()

function checkAddon(addon_path: string, pathDeep = 0) {
  /**
   *  查找文件 __init__.py,blender_manifest.toml
   *  如果有 blender_manifest.toml 则是新版本的插件
   */
  readDir(addon_path).then(async (entry) => {
    for (const file of entry) {
      if (file.isFile) {
        console.log(file, addon_path)
        if (file.name === "blender_manifest.toml") {
          blenderAddonStore.add_addon({is_extension: true, addon_path: addon_path, is_expand: false})
          return
        } else if (file.name === "__init__.py") {
          blenderAddonStore.add_addon({is_extension: false, addon_path: addon_path, is_expand: false})
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

function findAddon(addon_path: string) {
  // 1.检查是否为插件
  // 2.检查是否为插件文件夹
  lstat(addon_path).then(fi => {
    if (fi.isDirectory) {
      checkAddon(addon_path)
    }
  })
}

listen("tauri://drag-drop", async (event: Event<WatchEvent>) => {
  event.payload.paths.forEach((path) => {
    findAddon(path)
  })
})

</script>

<template>
  <template v-if="blenderAddonStore.addon_list.length !== 0">
    <v-list style="overflow: auto">
      <template v-for="(addon,index,) in blenderAddonStore.addon_list">
        <LinkAddonList
            :addon="addon"
        />
        <v-divider v-if="blenderAddonStore.addon_list.length-1 !== index"></v-divider>
      </template>
    </v-list>
  </template>
  <template v-else>
    <v-col>
      <p>Drag Addon Folder Here</p>
    </v-col>
  </template>
</template>

<style scoped>

</style>
