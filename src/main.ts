import {createApp} from "vue";
import App from "./App.vue";

import ToastService from 'primevue/toastservice';
import PrimeVue from 'primevue/config';

let app = createApp(App);

import {createPinia} from 'pinia'
import piniaPluginPersistedState from 'pinia-plugin-persistedstate';

const pinia = createPinia()
pinia.use(piniaPluginPersistedState)


app.use(pinia)
app.use(PrimeVue, {});
app.use(ToastService);

app.mount("#app");
