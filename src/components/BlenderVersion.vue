<script setup lang="ts">
import useBlenderAddonStore from "../stores.js";
import AddBlenderVersion from "@/components/AddBlenderVersion.vue";

const user_blender = useBlenderAddonStore()

</script>

<template>
  <v-card>
    <v-card-title>Blender Version</v-card-title>
    <v-card-text>
      <v-row>
        <div v-if="user_blender.blender_version_list.length === 0">
          <v-btn text="Reload" @click="user_blender.restore_blender_version()"/>
        </div>
        <v-hover
            v-slot="{ isHovering, props }"
            v-for="i in user_blender.sort_blender_version_list"
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
    <AddBlenderVersion/>
  </v-card>
</template>

<style scoped>

</style>