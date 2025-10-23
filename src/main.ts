import {createApp} from "vue";
import App from "./App.vue";

import PrimeVue from 'primevue/config';
import Aura from '@primevue/themes/aura';
import ToastService from 'primevue/toastservice';
import {createPinia} from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'

let app = createApp(App);


const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)

app.use(pinia)
app.use(PrimeVue, {theme: {preset: Aura}});
app.use(ToastService);
app.mount("#app");
