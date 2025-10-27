<script setup lang="ts">
import {useBlenderAddonStore} from "../stores.js";
import {computed} from "vue";

const userBlender = useBlenderAddonStore()

const list = computed(() => {
  return userBlender.blender_version_list
})
</script>

<template>
  <v-card>
    <v-card-title>Blender Version</v-card-title>
    <v-card-text>
      {{ userBlender.blender_version_list }}
      {{ userBlender.version_list }}
      <div v-if="userBlender.blender_version_list.length === 0">
        <v-btn text="Reload" @click="userBlender.restore_blender_version()"/>
      </div>
      <template v-for="(i,index) in userBlender.version_list">
        <v-chip
            class="ma-2"
            color="primary"
            closable
            v-tooltip="{ text: 'Right del version', location: 'bottom' }"
            @click:close="userBlender.remove_blender_version(String(i))"
        >
          {{ i }}
          {{ userBlender.version_list }} {{ index }}
        </v-chip>
      </template>
    </v-card-text>
    <v-card-actions>
      <v-row>
      </v-row>
      <v-btn>Clear</v-btn>
    </v-card-actions>
  </v-card>
</template>

<style scoped>

</style>