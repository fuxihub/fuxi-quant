import { createApp } from "vue";
import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import Ripple from 'primevue/ripple';
import Tooltip from 'primevue/tooltip';
import 'primeicons/primeicons.css';
import '@/style.css';
import App from '@/App.vue';
import router from '@/router';

const app = createApp(App);

app.use(router);

app.use(PrimeVue, {
    ripple: true,
    theme: {
        preset: Aura,
        options: {
            prefix: 'p',
            darkModeSelector: '.app-dark',
            // TailwindCSS v4 集成：设置 CSS Layer
            cssLayer: {
                name: 'primevue',
                order: 'theme, base, primevue'
            }
        }
    }
});

app.directive('ripple', Ripple);
app.directive('tooltip', Tooltip);

// 加载模型后显示应用

const showApp = () => {
    const logo = document.querySelector('.loading-logo');
    if (logo) {
        logo.classList.add('fade-out');
        setTimeout(() => app.mount("#app"), 800);
    } else {
        app.mount("#app");
    }
};

showApp()