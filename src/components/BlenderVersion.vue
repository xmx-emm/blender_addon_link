<script setup lang="ts">
import useBlenderAddonStore from "../stores.js";
import AddBlenderVersion from "@/components/AddBlenderVersion.vue";
import {computed} from "vue";

const user_blender = useBlenderAddonStore()
const sort_blender_version_list = computed(() => {
  return user_blender.blender_version_list.sort((a, b) => a.localeCompare(b));
})
</script>

<template>
  <v-card>
    <v-card-title>
      <p>
      Blender Version
      <AddBlenderVersion/>
      </p>
    </v-card-title>
    <v-card-text>
      <v-row>
        <div v-if="user_blender.blender_version_list.length === 0">
          <v-btn text="Reload" @click="user_blender.restore_blender_version()"/>
        </div>
        <v-hover
            v-slot="{ isHovering, props }"
            v-for="i in sort_blender_version_list"
        >
          <v-chip
              v-bind="props"
              class="ma-2"
              color="primary"
              :key="i"
              :closable="isHovering || false"
              @click:close="user_blender.remove_blender_version(String(i))"
          >
            {{ i }}
          </v-chip>
        </v-hover>
        <v-spacer></v-spacer>
      </v-row>
    </v-card-text>
  </v-card>
</template>

<style scoped>

</style>