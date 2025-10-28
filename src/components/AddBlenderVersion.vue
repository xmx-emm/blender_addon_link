<script setup lang="ts">

import {ref} from "vue";
import useBlenderAddonStore from "@/stores.ts";

const add_version = ref("5.1")

const otherAddonVersion = ['4.0', '4.1', '4.2', '4.3', '4.4', '4.5', '5.0', '5.1'];

const rules = {
  required: (value: string) => !!value || 'Required.',
  chars: (value: string) => value.length <= 3 || 'Max 3 characters',
  bv: (value: string) => {
    const pattern = /^\d+.\d+$/
    return pattern.test(value) || 'Invalid blender version.'
  },
}
const user_blender = useBlenderAddonStore()
</script>

<template>
  <v-bottom-sheet inset>
    <template v-slot:activator="{ props: activatorProps }">
      <v-fab
          v-bind="activatorProps"
          color="primary"
          icon="mdi-plus"
          variant="tonal"
          location="top end"
          size="small"
          absolute
          class="ma-9"
          offset
      >
      </v-fab>
    </template>
    <v-sheet>
      <v-list>
        <v-row>
          <template v-for="i in otherAddonVersion">
            <v-chip
                :key="i"
                @click="user_blender.add_blender_version(i)"
                v-if="!user_blender.blender_version_list.includes(i)"
            >
              {{ i }}
            </v-chip>
          </template>
        </v-row>
        <v-text-field v-model="add_version"
                      :rules="[rules.required, rules.bv,rules.chars]"
                      v-ripple
        ></v-text-field>
        <v-btn @click="user_blender.add_blender_version(add_version)"
               :disabled="user_blender.blender_version_list.includes(add_version)"
        >Add{{ add_version }}
        </v-btn>
      </v-list>
    </v-sheet>
  </v-bottom-sheet>

</template>

<style scoped>

</style>