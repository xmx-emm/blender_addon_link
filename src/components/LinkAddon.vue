<!--<script setup lang="ts">-->

<!--import {onBeforeMount, ref} from "vue";-->
<!--import {BaseDirectory, basename, join} from "@tauri-apps/api/path";-->
<!--import {exists, remove} from "@tauri-apps/plugin-fs";-->
<!--import {invoke} from "@tauri-apps/api/core";-->
<!--import {getAddonLinkFolder} from "@/utils/addon.ts";-->


<!--const props = defineProps({-->
<!--  addonPath: String,-->
<!--  isExtension: Boolean-->
<!--})-->


<!--const addonInfo = ref({-->
<!--  name: 'unknown',-->
<!--  linkVersion: []-->
<!--})-->

<!--function linkAddon(addon) {-->
<!--  const args = {to: addon.installFolder, from: props.addonPath}-->
<!--  invoke("link_dir", args)-->
<!--      .then(() => {-->
<!--        addon.isLink = true-->
<!--        const summary = addonInfo.value.name + ' Link'-->
<!--        const detail = addon.installFolder-->
<!--        console.log(addon)-->
<!--        console.log(summary, detail)-->
<!--        // toast.add({severity: 'success', summary: summary, detail: detail, life: 3000});-->
<!--      })-->
<!--}-->

<!--function unlinkAddon(addon) {-->
<!--  remove(addon.installFolder).then(() => {-->
<!--    addon.isLink = false-->
<!--    const summary = addonInfo.value.name + ' Unlink'-->
<!--    const detail = addon.installFolder-->
<!--    // toast.add({severity: 'warn', summary: summary, detail: detail, life: 3000});-->
<!--  })-->
<!--}-->

<!--onBeforeMount(() => {-->
<!--  basename(props.addonPath).then(async (name) => {-->
<!--    addonInfo.value.name = name-->
<!--    for (const version of linkAddonVersion) {-->
<!--      const addonFolder = await getAddonLinkFolder(version, props.isExtension)-->
<!--      const linkAddonPath = await join(addonFolder, name)-->

<!--      addonInfo.value.linkVersion.push({-->
<!--        version: version,-->
<!--        installFolder: linkAddonPath,-->
<!--        isLink: await exists(linkAddonPath)-->
<!--      })-->
<!--      await exists('avatar.png', {dir: BaseDirectory.AppData})-->
<!--    }-->
<!--  })-->
<!--})-->

<!--</script>-->

<!--<template>-->
<!--  <div class="flex flex-col">-->
<!--    <Panel :header="addonInfo.name" toggleable>-->
<!--      <p>Path: {{ addonPath }}</p>-->
<!--      <template v-for="addon in addonInfo.linkVersion">-->
<!--        <div>-->
<!--          {{ addon.version }}-->
<!--          <Button @click="linkAddon(addon)" text v-if="!addon.isLink">-->
<!--            Link-->
<!--          </Button>-->
<!--          <Button @click="unlinkAddon(addon)" text severity="danger" v-else>-->
<!--            Unlink-->
<!--          </Button>-->
<!--        </div>-->
<!--      </template>-->
<!--    </Panel>-->
<!--  </div>-->
<!--</template>-->

<!--<style scoped>-->

<!--</style>-->
