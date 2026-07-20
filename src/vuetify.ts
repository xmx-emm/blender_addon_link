import * as components from 'vuetify/components';
import * as directives from 'vuetify/directives';
import {createVuetify} from 'vuetify';
import {aliases, mdi} from 'vuetify/iconsets/mdi';

const vuetify = createVuetify({
    components,
    directives,
    theme: {
        defaultTheme: 'dark',
        themes: {
            dark: {
                dark: true,
                colors: {
                    primary: '#E87D0D', // Blender 橙
                    secondary: '#5A7D9A',
                    background: '#161619',
                    surface: '#1f1f26',
                    'surface-light': '#2a2a33',
                    'surface-variant': '#33333d',
                    error: '#EF5350',
                    success: '#66BB6A',
                    warning: '#FFA726',
                    info: '#4FC3F7',
                },
            },
        },
    },
    icons: {
        defaultSet: 'mdi',
        aliases,
        sets: {
            mdi,
        },
    },
    defaults: {
        VCard: {
            elevation: 0,
        },
        VBtn: {
            style: 'text-transform: none; letter-spacing: normal;',
        },
        VDialog: {
            maxWidth: 720,
        },
        VTooltip: {
            location: 'bottom',
        },
    },
});
export default vuetify;
