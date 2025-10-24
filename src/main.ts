import {createApp} from "vue";
import App from "./App.vue";

import ToastService from 'primevue/toastservice';
import PrimeVue from 'primevue/config';
import Aura from '@primevue/themes/aura';

let app = createApp(App);

import {createPinia} from 'pinia'
import piniaPluginPersistedState from 'pinia-plugin-persistedstate';

const pinia = createPinia()
pinia.use(piniaPluginPersistedState)


app.use(pinia)
app.use(PrimeVue, {theme: {preset: Aura}});
app.use(ToastService);

app.mount("#app");
