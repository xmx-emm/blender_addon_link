<script setup lang="ts">
import BlenderVersion from "@/components/BlenderVersion.vue";
import BlenderLink from "@/components/BlenderLink.vue";
import useBlenderAddonStore from "@/stores.ts";

const user_blender = useBlenderAddonStore()
</script>

<template>
  <v-app class="not_select" style="height: 100%">
    <BlenderVersion/>
    <BlenderLink/>
<!--    <div style="position: relative">-->
<!--    </div>-->
    <v-fab
        @click="user_blender.clear_addon()"
        color="primary"
        icon="mdi-close"
        location="right bottom"
        size="x-small"
        absolute
    >
    </v-fab>
  </v-app>
</template>

<style scoped>
</style>
<style>
</style>
