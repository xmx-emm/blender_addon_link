<script setup>

import {onBeforeMount, ref} from "vue";
import {appDataDir, BaseDirectory, basename, dirname, join} from "@tauri-apps/api/path";
import {exists, remove} from "@tauri-apps/plugin-fs";
import {invoke} from "@tauri-apps/api/core";

import {useToast} from "primevue/usetoast";

const toast = useToast();
const props = defineProps({
  addonPath: String,
  isExtension: Boolean
})


const linkAddonVersion = ['4.0', '4.1', '4.2', '4.3', '4.4', '4.5', '5.0', '5.1']
const addonInfo = ref({
  name: 'unknown',
  linkVersion: []
})

function addonExtension(version) {
  const vn = parseInt(version.replace(".", ""))
  return vn > 41;
}

async function getAddonLinkFolder(version, isExtension) {
  /**
   * 通过版本获取插件文件夹路径
   * 4.1没有扩展文件夹,4.2 >= 才有扩展
   * blender版本目前只有两位数 2024-10-28
   * C:\Users\Name\AppData\Roaming\Blender Foundation\Blender\4.4\extensions\user_default
   * C:\Users\Name\AppData\Roaming\Blender Foundation\Blender\4.4\script\addons
   */
  const extension = addonExtension(version)
  const pathJoin = [
    await dirname(await dirname(await appDataDir())),
    // 'AppData',
    'Roaming',
    'Blender Foundation',
    'Blender',
    version
  ]

  if (isExtension && extension) {
    pathJoin.push('extensions', 'user_default')
  } else {
    pathJoin.push('scripts', 'addons')
  }
  return await join(...pathJoin)
}

function linkAddon(addon) {
  const args = {to: addon.installFolder, from: props.addonPath}
  invoke("link_dir", args)
      .then(() => {
        addon.isLink = true
        const summary = addonInfo.value.name + ' Link'
        const detail = addon.installFolder
        console.log(addon)
        console.log(summary, detail)
        toast.add({severity: 'success', summary: summary, detail: detail, life: 3000});
      })
}

function unlinkAddon(addon) {
  remove(addon.installFolder).then(() => {
    addon.isLink = false
    const summary = addonInfo.value.name + ' Unlink'
    const detail = addon.installFolder
    toast.add({severity: 'warn', summary: summary, detail: detail, life: 3000});
  })
}

onBeforeMount(() => {
  basename(props.addonPath).then(async (name) => {
    addonInfo.value.name = name
    for (const version of linkAddonVersion) {
      const addonFolder = await getAddonLinkFolder(version, props.isExtension)
      const linkAddonPath = await join(addonFolder, name)

      addonInfo.value.linkVersion.push({
        version: version,
        installFolder: linkAddonPath,
        isLink: await exists(linkAddonPath)
      })
      await exists('avatar.png', {dir: BaseDirectory.AppData})
    }
  })
})

</script>

<template>
  <div class="flex flex-col">
    <Panel :header="addonInfo.name" toggleable>
      <p>Path: {{ addonPath }}</p>
      <template v-for="addon in addonInfo.linkVersion">
        <div>
          {{ addon.version }}
          <Button @click="linkAddon(addon)" text v-if="!addon.isLink">
            Link
          </Button>
          <Button @click="unlinkAddon(addon)" text severity="danger" v-else>
            Unlink
          </Button>
        </div>
      </template>
    </Panel>
  </div>
</template>

<style scoped>

</style>
