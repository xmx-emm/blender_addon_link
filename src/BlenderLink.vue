<script setup lang="ts">
// 拖入文件夹设置文件夹
// 读取那些Blender版本被Link了的,或者是已经有了的
// 功能:
// 1.检查是否为Link文件夹，取消Link
// 2.添加Link，如果不是Link的可以删除重新Link
// 4.列出可添加版本

import {ref} from "vue";
import LinkAddon from "./components/LinkAddon.vue";
import {listen} from "@tauri-apps/api/event";
import {lstat, readDir} from "@tauri-apps/plugin-fs";
import {join} from "@tauri-apps/api/path";

const addons = ref([]);// {"isExtension":False, path:"",id}


listen("tauri://drag-drop", async (event) => {
  event.payload.paths.forEach((path) => {
    findAddon(path)
  })
})

function clear() {
  addons.value = []
}

</script>

<template>
  <div>
    <div v-if="addons.length !== 0">
      <Button label="Clear" @click="clear">Clear</Button>
      <p>Test Blender Link</p>
      <div v-for="addon in addons">
        <LinkAddon :addonPath="addon.path" :is-extension="addon.isExtension"/>
      </div>
    </div>
    <div v-else>
      <p>Drag Addon Folder Here</p>
    </div>
  </div>
</template>

<style scoped>

</style>
