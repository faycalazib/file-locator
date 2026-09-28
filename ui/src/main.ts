import './styles/fonts.css';
import './styles/tokens.css';
import './styles/themes/crayons.css';
import './styles/themes/sonar.css';
import './styles/base.css';
import './styles/print.css';

import { mount } from 'svelte';
import App from './App.svelte';
import { i18n } from './lib/i18n/index.svelte';
import { ui } from './lib/stores/ui.svelte';

i18n.init();
// Before mount: the right theme from the first frame (no flash).
ui.initTheme();

const target = document.getElementById('app');
if (!target) throw new Error('#app container missing from index.html');

export default mount(App, { target });
