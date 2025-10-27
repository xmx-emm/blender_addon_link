<script setup lang="ts">
import {useBlenderAddonStore} from "../stores.js";
import {ref} from "vue";

const user_blender = useBlenderAddonStore()
const add_version = ref("5.1")

const rules = {
  required: (value: string) => !!value || 'Required.',
  counter: (value: string) => value.length <= 3 || 'Max 3 characters',
  email: (value: string) => {
    const pattern = /^\d+.\d+$/
    return pattern.test(value) || 'Invalid blender version.'
  },
}

</script>

<template>
  <v-card>
    <v-card-title>Blender Version</v-card-title>
    <v-card-text>
      <div v-if="user_blender.blender_version_list.length === 0">
        <v-btn text="Reload" @click="user_blender.restore_blender_version()"/>
      </div>
      <v-chip
          v-for="i in user_blender.blender_version_list"
          class="ma-2"
          color="primary"
          :key="i"
          closable
          @click:close="user_blender.remove_blender_version(String(i))"
      >
        {{ i }}
      </v-chip>
    </v-card-text>
    <v-card-actions>
      <v-text-field :model-value="add_version"
                    :rules="[rules.required, rules.email,rules.counter]"
                    v-ripple
      ></v-text-field>
      <v-btn @click="user_blender.add_blender_version(add_version)">Add{{add_version}}</v-btn>
    </v-card-actions>
  </v-card>
</template>

<style scoped>

</style>