<script setup lang="ts">

import {onBeforeMount, ref} from "vue";
import {basename, join} from "@tauri-apps/api/path";
import {exists} from "@tauri-apps/plugin-fs";
import {getAddonLinkFolder} from "@/utils/addon.ts";
import useBlenderAddonStore from "@/stores.ts";
import {AddonLinkItem} from "@/data.ts";
import {invoke} from "@tauri-apps/api/core";


const props = defineProps({
  addon_path: String,
  is_extension: Boolean
})


const userBlender = useBlenderAddonStore()
const addon_name = ref("unknown")

const link_addon_list = ref<AddonLinkItem[]>([])


onBeforeMount(async () => {
  addon_name.value = await basename(<string>props.addon_path)
  link_addon_list.value = []
  try {
    for (const version of userBlender.blender_version_list) {
      await checkAddon(version)
    }
  } catch (e) {
    console.log("link_addon_list error", e, props,)
  }
})

async function checkAddon(version: string) {
  const addon_folder = await getAddonLinkFolder(version, props.is_extension)
  const link_addon_path = await join(addon_folder, addon_name.value)
  const is_exists = await exists(link_addon_path);
  let is_sym_link = false;
  if (is_exists) {
    try {
      is_sym_link = await invoke("is_symbolic_link", {path: link_addon_path});
    } catch (e) {
      console.log("checkAddon error", version, e)
    }
  }

  const item = {
    blender_version: version,
    install_folder: link_addon_path,
    is_exists: is_exists,
    is_symbolic_link: is_sym_link,
  }
  const index = link_addon_list.value.findIndex(f => f.blender_version === version)
  if (index !== -1) {
    link_addon_list.value.splice(index, 1, item)
  } else {
    link_addon_list.value.push(item)
  }
}

function link(addon: AddonLinkItem) {
  const args = {from: props.addon_path || "unknown", to: addon.install_folder}
  invoke("link_dir", args).then(() => {
    checkAddon(addon.blender_version)
  })
}

async function unlink(addon: AddonLinkItem) {
  if (await exists(addon.install_folder)) {
    invoke("unlink_dir", {ud: addon.install_folder}).then(() => {
      checkAddon(addon.blender_version)
    })
  }
}

function click(addon: AddonLinkItem) {
  checkAddon(addon.blender_version)
  console.log("click", addon)
  if (addon.is_exists || addon.is_symbolic_link) {
    unlink(addon)
  } else {
    link(addon)
  }
}

</script>

<template>
  <div class="flex flex-col">
    <v-card :title="addon_name" toggleable>
      <v-card-subtitle>Path: {{ props.addon_path }}</v-card-subtitle>
      <v-card-text>
        <v-chip
            v-for="addon in link_addon_list.sort((a, b) => a.blender_version.localeCompare(b.blender_version))"
            @click="click(addon)"
            style="margin-right: 10px"
            :append-icon="addon.is_exists ? (addon.is_symbolic_link ? 'mdi-check': 'mdi-exclamation'): 'mdi-close'"
            v-tooltip="{text: (addon.is_exists ? (addon.is_symbolic_link ? 'Link addon': 'Remove addon!!'): 'Unlink addon!')+' ' +addon.blender_version, location:'bottom'}"
            :color="addon.is_exists ? (addon.is_symbolic_link ? 'green': 'red'): 'secondary'"
        >
          {{ addon.blender_version }}
        </v-chip>
        <v-spacer></v-spacer>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>

</style>
