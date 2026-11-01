import { mount } from 'svelte';
import '@fontsource/dela-gothic-one/cyrillic-400.css';
import '@fontsource/dela-gothic-one/latin-400.css';
import '@fontsource-variable/onest/index.css';
import './app.css';
import App from './App.svelte';

export default mount(App, { target: document.getElementById('app')! });
