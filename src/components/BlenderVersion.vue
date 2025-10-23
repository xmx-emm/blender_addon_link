<script setup lang="ts">
import {useBlenderAddonStore} from "../stores.js";

const userBlender = useBlenderAddonStore()

</script>

<template>
  <Card>
    <template #title>Blender Version</template>
    <template #content>
      <div v-if="userBlender.blender_version_list.length === 0">
        <Button label="Reload" severity="secondary" variant="text" @click="userBlender.restore_blender_version()"/>
      </div>
      <div v-else>
        <Button v-for="i in userBlender.blender_version_list"
                variant="link"
                @contextmenu.prevent="userBlender.remove_blender_version(i)"
                v-tooltip.bottom="{ value: 'Right del version', showDelay: 200, hideDelay: 20, placeholder: 'Bottom' }"
        >
          {{ i }}
        </Button>
      </div>
    </template>
  </Card>

</template>

<style scoped>

</style>