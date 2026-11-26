import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

// The real visible size in px: 100vh also counts the area under the browser's
// address bar, which would push the sideways layout off the screen.
function measure() {
  const root = document.documentElement.style;
  root.setProperty('--screen-w', `${window.innerWidth}px`);
  root.setProperty('--screen-h', `${window.innerHeight}px`);
}
measure();
window.addEventListener('resize', measure);

mount(App, { target: document.getElementById('app')! });

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {});
}
