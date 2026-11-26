import { mount } from 'svelte';
import { setLanguage } from '../lib/i18n.svelte';
import Editor from './Editor.svelte';

// The editor's texts are looked up once while it is being drawn, so the
// language has to be known first: the one chosen in the settings, or the
// browser's own.
async function start() {
  try {
    const config = await (await fetch('/api/config')).json();
    setLanguage(config.language);
  } catch {
    setLanguage(null);
  }
  mount(Editor, { target: document.getElementById('app')! });
}

start();
