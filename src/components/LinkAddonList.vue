<script setup lang="ts">

import {onBeforeMount, ref} from "vue";
import {basename, join} from "@tauri-apps/api/path";
import {exists} from "@tauri-apps/plugin-fs";
import {getAddonLinkFolder} from "@/utils/addon.ts";
import useBlenderAddonStore from "@/stores.ts";
import {AddonLinkItem} from "@/data.ts";
import {invoke} from "@tauri-apps/api/core";
import LinkItem from "@/components/LinkItem.vue";


const props = defineProps(["addon"])


const user_blender = useBlenderAddonStore()
const addon_name = ref("unknown")

const link_addon_list = ref<AddonLinkItem[]>([])

async function checkAddon(version: string) {
  const addon_folder = await getAddonLinkFolder(version, props.addon.is_extension)
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
  const index = link_addon_list.value.findIndex((f: AddonLinkItem) => f.blender_version === version)
  if (index !== -1) {
    link_addon_list.value.splice(index, 1, item)
  } else {
    link_addon_list.value.push(item)
  }
}

onBeforeMount(async () => {
  addon_name.value = await basename(<string>props.addon.addon_path)
  link_addon_list.value = []
  try {
    for (const version of user_blender.blender_version_list) {
      await checkAddon(version)
    }
  } catch (e) {
    console.log("link_addon_list error", e, props,)
  }
})


</script>

<template>
  <div class="flex flex-col">
    <v-card toggleable>
      <v-card-title>
        <div class="d-flex flex-row align-items-center">
          {{ addon_name }}
          <v-icon :icon="props.addon.is_expand ? 'mdi-chevron-down' : 'mdi-chevron-up'"
                  @click="props.addon.is_expand = !props.addon.is_expand"
          />
          <v-spacer></v-spacer>

          <template v-if="!addon.is_expand">
            <LinkItem :addon="addon" :link_addon_list="link_addon_list" :addon_name="addon_name"
                      @check-addon="checkAddon"></LinkItem>
          </template>
        </div>
      </v-card-title>
      <v-card-subtitle v-if="addon.is_expand">Path: {{ props.addon.addon_path }}</v-card-subtitle>
      <v-card-text v-if="addon.is_expand">
        <LinkItem :addon="addon" :link_addon_list="link_addon_list" :addon_name="addon_name"
                  @check-addon="checkAddon"></LinkItem>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>

</style>
