<script setup lang="ts">

import {onBeforeMount, onMounted, ref} from "vue";
import {basename, join} from "@tauri-apps/api/path";
import {exists, lstat} from "@tauri-apps/plugin-fs";
import {getAddonLinkFolder} from "@/utils/addon.ts";
import useBlenderAddonStore from "@/stores.ts";
import {AddonLinkItem} from "@/data.ts";


const props = defineProps({
  addon_path: String,
  is_extension: Boolean
})


const userBlender = useBlenderAddonStore()
const addon_name = ref("unknown")

const link_addon_list = ref<AddonLinkItem[]>([])

onMounted(async () => {
      try {
        link_addon_list.value = []
        for (const version of userBlender.blender_version_list) {
          const addonFolder = await getAddonLinkFolder(version, props.is_extension)
          const linkAddonPath = await join(addonFolder, addon_name.value)
          const state = await lstat(linkAddonPath);
          link_addon_list.value.push({
            blender_version: version,
            install_folder: linkAddonPath,
            is_exists: await exists(linkAddonPath),
            is_symbolic_link: state.isSymlink
          })
        }
      } catch (e) {
        console.log("ee", e)
      }
    }
)

onBeforeMount(async () => {
  addon_name.value = await basename(<string>props.addon_path)
})

function link(addon: AddonLinkItem) {
  console.log("link", addon)
}

function unlink(addon: AddonLinkItem) {
  console.log("unlink", addon)
}

</script>

<template>
  <div class="flex flex-col">
    <v-card :header="addon_name" toggleable>
      <v-card-text>
        <p>Path: {{ props.addon_path }}</p>
        {{ link_addon_list }}
        <div v-for="addon in link_addon_list">
          {{ addon.blender_version }}
          <v-btn @click="link(addon)" text v-if="!addon.is_exists">
            Link
          </v-btn>
          <v-btn @click="unlink(addon)" text severity="danger" v-else>
            Unlink
          </v-btn>
        </div>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>

</style>
