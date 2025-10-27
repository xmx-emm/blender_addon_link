import {createApp} from "vue";
import App from "./App.vue";

import {createPinia} from 'pinia'
import piniaPluginPersistedState from 'pinia-plugin-persistedstate';

//vuetify
import '@mdi/font/css/materialdesignicons.css';
import vuetify from "@/vuetify.ts";
import 'vuetify/styles';

let app = createApp(App);

const pinia = createPinia()
pinia.use(piniaPluginPersistedState)


app.use(pinia)
app.use(vuetify)
app.mount("#app");
