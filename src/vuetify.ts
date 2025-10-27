import * as components from 'vuetify/components';
import * as directives from 'vuetify/directives';
import {createVuetify} from 'vuetify';
import {aliases, mdi} from 'vuetify/iconsets/mdi';


const vuetify = createVuetify({
    components,
    directives,
    theme: {
        defaultTheme: 'dark', // 'light' | 'dark' | 'system'
    },
    icons: {
        defaultSet: 'mdi',
        aliases,
        sets: {
            mdi,
        },
    },
    defaults: {
        VDialog: {
            maxWidth: 1000,
        }
    }
});
export default vuetify;