<script setup lang="ts">
import {AddonLinkItem} from "@/data.ts";
import {invoke} from "@tauri-apps/api/core";
import {exists} from "@tauri-apps/plugin-fs";

const props = defineProps(["link_addon_list", "addon", "addon_name"])
const emits = defineEmits(["checkAddon"])
function link(addon: AddonLinkItem) {
  const args = {from: props.addon.addon_path || "unknown", to: addon.install_folder}
  invoke("link_dir", args).then(() => {
    emits("checkAddon", addon.blender_version)
  })
}

async function unlink(addon: AddonLinkItem) {
  if (await exists(addon.install_folder)) {
    invoke("unlink_dir", {ud: addon.install_folder}).then(() => {
      emits("checkAddon", addon.blender_version)
    }).catch((e) => {
      console.log("unlink_dir error", e)
    })
  }
}

function click(addon: AddonLinkItem) {
  emits("checkAddon", addon.blender_version)
  console.log("click", addon)
  if (addon.is_exists || addon.is_symbolic_link) {
    unlink(addon)
  } else {
    link(addon)
  }
}
</script>

<template>
  <v-chip
      v-for="la in props.link_addon_list.sort((a:AddonLinkItem, b:AddonLinkItem) => a.blender_version.localeCompare(b.blender_version))"
      @click="click(la)"
      style="margin-right: 10px"
      :text="la.blender_version"
      :append-icon="la.is_exists ? (la.is_symbolic_link ? 'mdi-check': 'mdi-exclamation'): 'mdi-close'"
      v-tooltip="{text: (la.is_exists ? (la.is_symbolic_link ? 'Link addon': 'Remove addon!!'): 'Unlink addon!')+' ' +la.blender_version, location:'bottom'}"
      :color="la.is_exists ? (la.is_symbolic_link ? 'green': 'red'): 'secondary'"
  >
  </v-chip>
</template>

<style scoped>

</style>