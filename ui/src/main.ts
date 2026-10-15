import './styles/fonts.css';
import './styles/tokens.css';
import './styles/themes/crayons.css';
import './styles/themes/sonar.css';
import './styles/themes/clay.css';
import './styles/themes/carnet.css';
import './styles/themes/scrapbook.css';
import './styles/themes/soft.css';
import './styles/themes/brush.css';
import './styles/themes/electric.css';
import './styles/themes/gold.css';
import './styles/themes/leafora.css';
import './styles/themes/potea.css';
import './styles/themes/neural.css';
import './styles/themes/nebula.css';
import './styles/themes/lagoon.css';
import './styles/themes/cosmo.css';
import './styles/themes/greenhouse.css';
import './styles/themes/noor.css';
import './styles/themes/orchard.css';
import './styles/themes/candy.css';
import './styles/themes/cocoon.css';
import './styles/themes/marine.css';
import './styles/themes/daylight.css';
import './styles/themes/cloud.css';
import './styles/themes/lavender.css';
import './styles/themes/frost.css';
import './styles/themes/starry.css';
import './styles/themes/aurora.css';
import './styles/themes/sport.css';
import './styles/themes/fleet.css';
import './styles/themes/command.css';
import './styles/themes/telemetry.css';
import './styles/themes/onyx.css';
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
