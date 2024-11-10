import {createApp} from "vue";
import App from "./App.vue";

import PrimeVue from 'primevue/config';
import Aura from '@primevue/themes/aura';
import ToastService from 'primevue/toastservice';

let app = createApp(App);
app.mount("#app");
app.use(PrimeVue, {
    theme: {
        preset: Aura
    }
});
app.use(ToastService);
