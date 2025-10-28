<script setup lang="ts">
import BlenderVersion from "@/components/BlenderVersion.vue";
</script>

<template>
  <v-app class="not_select not_scrolling">
    <BlenderVersion/>
  </v-app>
  <!--  <BlenderLink/>-->

  <!--      <v-card-actions v-if=" isHovering">-->
  <!--        <input v-model="add_version"-->
  <!--                      :rules="[rules.required, rules.email,rules.counter]"-->
  <!--                      v-ripple-->
  <!--                      style="max-height: 20px"-->
  <!--        ></input>-->
  <!--        <v-btn @click="user_blender.add_blender_version(add_version)"-->
  <!--               :disabled="user_blender.blender_version_list.includes(add_version)"-->
  <!--        >Add{{ add_version }}-->
  <!--        </v-btn>-->
  <!--      </v-card-actions>-->
</template>

<style scoped>
</style>
<style>
</style>
