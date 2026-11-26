<script lang="ts">
  import { iconNames, icons } from '../lib/icons';
  import {
    assetUrl,
    palette,
    presetNames,
    resolveTheme,
    type Background,
    type CardLook,
    type Colors,
    type ThemeDef,
  } from '../lib/theme';
  import type { Action, Config, DeckButton, KeyLook, NightMode, PageDef, WidgetDef } from '../lib/types';
  import Layout, { type Rect } from './Layout.svelte';
  import { LANGUAGES, language } from '../lib/i18n.svelte';
  import { t } from './strings';

  type Asset = { name: string; kind: 'image' | 'font'; size: number };

  let cfg = $state<Config | null>(null);
  let assets = $state<Asset[]>([]);
  let status = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let problem = $state('');
  let tab = $state('layout');

  const tabs = [
    ['layout', t('Rozložení')],
    ['theme', t('Motiv')],
    ['colors', t('Barvy')],
    ['cards', t('Karty')],
    ['wall', t('Pozadí')],
    ['font', t('Písmo')],
    ['pages', t('Stránky')],
    ['keys', t('Tlačítka')],
    ['night', t('Noc')],
    ['link', t('Připojení')],
  ];

  const images = $derived(assets.filter((a) => a.kind === 'image'));
  const fonts = $derived(assets.filter((a) => a.kind === 'font'));

  // ---- loading and saving -------------------------------------------------

  let lastSaved: string | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function api(path: string, init?: RequestInit) {
    const res = await fetch(path, init);
    if (!res.ok) {
      const body = await res.json().catch(() => null);
      throw new Error(body?.error ?? `chyba ${res.status}`);
    }
    return res;
  }

  async function loadAssets() {
    assets = await (await api('/api/assets')).json();
  }

  async function load() {
    try {
      const loaded = await (await api('/api/config')).json();
      lastSaved = JSON.stringify(loaded);
      cfg = loaded;
      await loadAssets();
    } catch (err) {
      status = 'error';
      problem = t('Agent neodpovídá ({m}).', { m: (err as Error).message });
    }
  }
  load();

  async function save(body: string) {
    status = 'saving';
    try {
      await api('/api/config', { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body });
      lastSaved = body;
      status = 'saved';
    } catch (err) {
      status = 'error';
      problem = (err as Error).message;
    }
  }

  // Every change is saved a moment later and shows up on the phone by itself.
  $effect(() => {
    if (!cfg) return;
    const body = JSON.stringify(cfg);
    if (body === lastSaved) return;
    clearTimeout(timer);
    timer = setTimeout(() => save(body), 250);
  });

  async function chooseLanguage(code: string) {
    if (!cfg) return;
    cfg.language = code;
    clearTimeout(timer);
    await save(JSON.stringify(cfg));
    location.reload();
  }

  async function reset() {
    if (!confirm(t('Vrátit celé nastavení (stránky, tlačítka i vzhled) do výchozího stavu?'))) return;
    const fresh = await (await api(`/api/reset?lang=${language()}`, { method: 'POST' })).json();
    lastSaved = JSON.stringify(fresh);
    cfg = fresh;
    picked = 0;
  }

  async function upload(e: Event, done: (name: string) => void) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      await api(`/api/assets/${encodeURIComponent(file.name)}`, { method: 'PUT', body: file });
      await loadAssets();
      done(file.name);
    } catch (err) {
      status = 'error';
      problem = t('Soubor se nepodařilo nahrát: {m}', { m: (err as Error).message });
    }
  }

  // ---- theme --------------------------------------------------------------

  const own = $derived(cfg?.themes ?? {});
  const look = $derived(resolveTheme(cfg?.theme, own));
  const themeNames = $derived([...presetNames, ...Object.keys(own).filter((n) => !presetNames.includes(n))]);
  const currentPreset = $derived(typeof cfg?.theme === 'string' ? cfg.theme : (cfg?.theme?.preset ?? 'midnight'));

  /** The global theme as an object we can edit. */
  function theme(): ThemeDef {
    const c = cfg!;
    if (typeof c.theme !== 'object' || c.theme === null) c.theme = c.theme ? { preset: c.theme } : {};
    return c.theme;
  }

  function pickPreset(name: string) {
    // A fresh start from that theme; the wallpaper and font are kept.
    const old = theme();
    cfg!.theme = { preset: name, ...(old.background !== undefined ? { background: old.background } : {}), ...(old.font ? { font: old.font } : {}) };
  }

  function setColor(key: keyof Colors, value: string | null) {
    const t = theme();
    const colors = { ...t.colors };
    if (value === null) delete colors[key];
    else colors[key] = value;
    t.colors = colors;
  }

  function setCard<K extends keyof CardLook>(key: K, value: CardLook[K]) {
    const t = theme();
    t.card = { ...t.card, [key]: value };
  }

  function background(): Background {
    const t = theme();
    if (typeof t.background === 'string') t.background = { image: t.background };
    else if (!t.background) t.background = { ...(look.background ?? {}) };
    return t.background as Background;
  }

  function setWall<K extends keyof Background>(key: K, value: Background[K] | undefined) {
    const bg = background();
    if (value === undefined || value === '') delete bg[key];
    else bg[key] = value;
  }

  function pickWall(name: string) {
    const bg = background();
    bg.image = name;
    // A bright picture needs some darkening for the text on it to stay readable.
    if (bg.dim === undefined) bg.dim = 0.3;
  }

  function saveAsTheme() {
    const name = prompt(t('Název vlastního motivu:'))?.trim();
    if (!name) return;
    const snapshot = JSON.parse(JSON.stringify(theme())) as ThemeDef;
    cfg!.themes = { ...cfg!.themes, [name]: snapshot };
    cfg!.theme = name;
  }

  function removeTheme(name: string) {
    const themes = { ...cfg!.themes };
    delete themes[name];
    cfg!.themes = themes;
    if (currentPreset === name) cfg!.theme = 'midnight';
  }

  const colorRows: [keyof Colors, string][] = [
    ['accent', t('Zvýraznění')],
    ['bg', t('Pozadí')],
    ['card', t('Karty')],
    ['cardHi', t('Karty – světlejší')],
    ['line', t('Okraje')],
    ['text', t('Text')],
    ['dim', t('Text – tlumený')],
    ['mute', t('Text – nejtlumenější')],
    ['ok', t('V pořádku')],
    ['warn', t('Varování')],
    ['bad', t('Chyba')],
  ];

  /** `<input type=color>` only understands #rrggbb. */
  const hex6 = (value: string) => {
    const v = palette[value] ?? value;
    if (/^#[0-9a-f]{6}/i.test(v)) return v.slice(0, 7);
    if (/^#[0-9a-f]{3}$/i.test(v)) return '#' + v.slice(1).replace(/./g, '$&$&');
    return '#888888';
  };

  const fontChoices: [string, string][] = [
    ['', t('Systémové')],
    ['serif', t('Patkové')],
    ['monospace', t('Strojové')],
    ["'Roboto Condensed', sans-serif-condensed, sans-serif", t('Zúžené')],
    ['cursive', t('Psací')],
  ];

  function setFont(key: 'family' | 'file', value: string) {
    const t = theme();
    const font = { ...t.font };
    if (value) font[key] = value;
    else delete font[key];
    t.font = font;
  }

  // ---- pages --------------------------------------------------------------

  function pageWall(page: PageDef): string {
    if (page.background === undefined) return '';
    if (page.background === null) return '-';
    return typeof page.background === 'string' ? page.background : (page.background.image ?? '');
  }

  function setPageWall(page: PageDef, value: string) {
    if (value === '') delete page.background;
    else if (value === '-') page.background = null;
    else page.background = { image: value, dim: 0.25 };
  }

  function setPageTheme(page: PageDef, value: string) {
    if (value) page.theme = value;
    else delete page.theme;
  }

  // ---- connection: USB and Wi-Fi --------------------------------------------

  type PairingInfo = { enabled: boolean; address: string | null; port: number; code: string; seconds: number; url: string | null; qr: string | null };
  type Paired = { id: string; name: string; created: number };
  let pairing = $state<PairingInfo | null>(null);
  let paired = $state<Paired[]>([]);

  async function loadLink(renew = false) {
    try {
      pairing = await (await api('/api/pairing', renew ? { method: 'POST' } : undefined)).json();
      paired = await (await api('/api/devices')).json();
    } catch {
      /* the status line already says the agent is unreachable */
    }
  }

  // While the tab is open, keep the code and the list fresh: a phone that has
  // just paired shows up, and a used or expired code is replaced.
  $effect(() => {
    if (tab !== 'link') return;
    void cfg?.wireless;
    loadLink();
    const timer = setInterval(loadLink, 3000);
    return () => clearInterval(timer);
  });

  async function forget(device: Paired) {
    if (!confirm(t('Zrušit spárování s „{name}“? Telefon se odpojí a bude se muset spárovat znovu.', { name: device.name }))) return;
    await api(`/api/devices/${device.id}`, { method: 'DELETE' });
    loadLink();
  }

  // ---- layout ------------------------------------------------------------

  const widgetNames: Record<string, string> = {
    clock: t('Hodiny'),
    media: t('Přehrávač'),
    system: t('Počítač'),
    deck: t('Tlačítka'),
    timer: t('Časovač'),
    todo: t('Úkoly'),
    processes: t('Procesy'),
    calendar: t('Kalendář'),
    ai: t('AI relace'),
    models: t('Lokální modely'),
    discord: 'Discord',
  };
  const widgetSizes: Record<string, [number, number]> = {
    clock: [4, 3],
    media: [5, 4],
    system: [3, 4],
    deck: [6, 2],
    timer: [3, 4],
    todo: [4, 4],
    processes: [4, 4],
    calendar: [4, 4],
    ai: [5, 4],
    models: [4, 4],
    discord: [4, 6],
  };

  // Where the agent's program is, for the commands other tools have to run.
  let agentExe = $state('streamphone-agent.exe');
  let dashboardPort = $state<number | null>(null);
  api('/api/info')
    .then((res) => res.json())
    .then((info) => {
      if (info.exe) agentExe = info.exe;
      // The port the agent really listens on (it can be overridden at start).
      if (info.port) dashboardPort = info.port;
    })
    .catch(() => {});

  let tool = $state('claude');
  const toolTabs = [
    ['claude', 'Claude Code'],
    ['codex', 'Codex'],
    ['other', t('Ostatní')],
  ];

  const codexSnippet = $derived.by(() => {
    const command = `"${agentExe}" hook --source codex`;
    const hooks: Record<string, unknown> = {};
    for (const name of ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'Stop', 'SessionEnd']) {
      hooks[name] = [{ hooks: [{ type: 'command', command, timeout: 5 }] }];
    }
    return JSON.stringify({ hooks }, null, 2);
  });
  const codexNotify = $derived(`notify = [${[agentExe, 'report', '--source', 'codex'].map((part) => JSON.stringify(part)).join(', ')}]`);
  const otherSnippet = $derived(
    [
      t('# Začátek práce, čekání na tebe a konec, z jakéhokoli skriptu nebo nástroje:'),
      `"${agentExe}" report --source aider --status working --title "Refactoring the cart"`,
      `"${agentExe}" report --source aider --status waiting --detail "Apply the changes?"`,
      `"${agentExe}" report --source aider --status done --detail "Done, 3 files"`,
      '',
      t('# Nástroj s hooky, který posílá událost jako JSON na standardní vstup:'),
      `"${agentExe}" hook --source gemini`,
    ].join('\n'),
  );

  // ---- local model servers ----

  type ServerDef = { name: string; url: string; kind: string };
  const knownServers: ServerDef[] = [
    { name: 'LM Studio', url: 'http://127.0.0.1:1234', kind: 'lmstudio' },
    { name: 'Ollama', url: 'http://127.0.0.1:11434', kind: 'ollama' },
  ];
  /** The widget's list; when it has none, the two usual servers are watched. */
  const serverList = $derived.by(() => (widget?.config?.servers as ServerDef[] | undefined) ?? knownServers);

  function setServers(list: ServerDef[]) {
    widget!.config = { ...widget!.config, servers: list };
  }
  function toggleServer(known: ServerDef, enabled: boolean) {
    const rest = serverList.filter((s) => s.url !== known.url);
    setServers(enabled ? [known, ...rest] : rest);
  }
  function editServer(index: number, key: 'name' | 'url', value: string) {
    setServers(serverList.map((s, i) => (i === index ? { ...s, [key]: value } : s)));
  }

  // What the user adds to Claude Code's settings so it reports to the agent.
  // The editor's own address is the right one: hooks go to the PC-only port.
  const hookEvents = ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'Notification', 'Stop', 'SessionEnd'];
  const hookSnippet = $derived.by(() => {
    // The numeric address: `localhost` can cost two seconds per call on Windows.
    const url = `http://127.0.0.1:${location.port}/api/hook`;
    const hooks: Record<string, unknown> = {};
    for (const name of hookEvents) hooks[name] = [{ hooks: [{ type: 'http', url, timeout: 5 }] }];
    const wait = Math.min(300, Math.max(5, Number(widget?.config?.wait ?? 45)));
    hooks.PermissionRequest = [{ hooks: [{ type: 'http', url, timeout: wait + 15 }] }];
    return JSON.stringify({ hooks }, null, 2);
  });
  let copied = $state(false);
  async function copyHooks() {
    await navigator.clipboard.writeText(hookSnippet);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function setNight<K extends keyof NightMode>(key: K, value: NightMode[K] | '' | undefined) {
    const night = { ...cfg!.night };
    if (value === '' || value === undefined) delete night[key];
    else night[key] = value as NightMode[K];
    cfg!.night = night;
  }

  let pageIndex = $state(0);
  let selectedId = $state<string | null>(null);
  let frame = $state<HTMLIFrameElement>();
  let phoneW = $state(0);
  let phoneH = $state(0);

  const page = $derived(cfg?.pages[Math.min(pageIndex, cfg.pages.length - 1)]);
  const widget = $derived(page?.widgets.find((w) => w.id === selectedId));

  /** Turns the preview to the page being laid out. */
  function showPage() {
    frame?.contentWindow?.postMessage({ type: 'streamphone:page', index: pageIndex }, '*');
  }
  $effect(() => {
    if (tab === 'layout' && cfg) {
      void pageIndex;
      showPage();
    }
  });

  function place(id: string, rect: Rect) {
    const w = page?.widgets.find((x) => x.id === id);
    if (w && (w.x !== rect.x || w.y !== rect.y || w.w !== rect.w || w.h !== rect.h)) Object.assign(w, rect);
  }

  const allIds = () => new Set((cfg?.pages ?? []).flatMap((pg) => [pg.id, ...pg.widgets.map((w) => w.id)]));

  /** `base`, `base-2`, `base-3`… whichever is still free. Ids are how the agent finds a button. */
  function freeId(base: string) {
    const used = allIds();
    if (!used.has(base)) return base;
    let n = 2;
    while (used.has(`${base}-${n}`)) n++;
    return `${base}-${n}`;
  }

  /** The first place on the page where a widget of this size fits without overlapping. */
  function freeSpot(pg: PageDef, w: number, h: number): Rect | null {
    for (let y = 0; y + h <= 6; y++) {
      for (let x = 0; x + w <= 12; x++) {
        const hit = pg.widgets.some((o) => x < o.x + o.w && o.x < x + w && y < o.y + o.h && o.y < y + h);
        if (!hit) return { x, y, w, h };
      }
    }
    return null;
  }

  function addWidget(type: string) {
    const pg = page!;
    const [w, h] = widgetSizes[type];
    const spot = freeSpot(pg, w, h) ?? freeSpot(pg, 2, 2) ?? freeSpot(pg, 1, 1) ?? { x: 0, y: 0, w, h };
    const fresh: WidgetDef = { id: freeId(`${pg.id}-${type}`), type, ...spot };
    if (type === 'deck') {
      fresh.config = { cols: 3, buttons: [{ label: t('Nové'), icon: 'star', color: 'blue', action: { do: 'media.play_pause' } }] };
    }
    pg.widgets.push(fresh);
    selectedId = fresh.id;
  }

  function removeWidget() {
    const pg = page!;
    pg.widgets = pg.widgets.filter((w) => w.id !== selectedId);
    selectedId = null;
  }

  function addPage() {
    const c = cfg!;
    c.pages.push({ id: freeId('page'), name: t('Nová stránka'), widgets: [] });
    pageIndex = c.pages.length - 1;
    selectedId = null;
  }

  function removePage() {
    const c = cfg!;
    if (c.pages.length < 2 || !confirm(t('Smazat stránku „{name}“ i s jejími widgety?', { name: page!.name }))) return;
    c.pages.splice(pageIndex, 1);
    pageIndex = Math.max(0, pageIndex - 1);
    selectedId = null;
  }

  function movePage(by: number) {
    const c = cfg!;
    const to = pageIndex + by;
    if (to < 0 || to >= c.pages.length) return;
    const [item] = c.pages.splice(pageIndex, 1);
    c.pages.splice(to, 0, item);
    pageIndex = to;
  }

  function setOption(key: string, value: unknown, fallback: unknown) {
    const w = widget!;
    const config = { ...w.config };
    if (value === fallback || value === '' || value === undefined) delete config[key];
    else config[key] = value;
    w.config = config;
  }

  function setStyle(key: 'accent' | 'text' | 'background' | 'opacity' | 'border', value: string | number | boolean | undefined) {
    const w = widget!;
    const style: Record<string, unknown> = { ...w.style };
    if (value === undefined || value === '') delete style[key];
    else style[key] = value;
    if (Object.keys(style).length) w.style = style;
    else delete w.style;
  }

  function editKeys() {
    deckIndex = Math.max(0, decks.findIndex((d) => d.widget.id === selectedId));
    picked = 0;
    tab = 'keys';
  }

  // ---- deck keys ----------------------------------------------------------

  const decks = $derived(
    (cfg?.pages ?? []).flatMap((page) => page.widgets.filter((w) => w.type === 'deck').map((widget) => ({ page, widget }))),
  );
  let deckIndex = $state(0);
  let picked = $state(0);
  const deck = $derived(decks[Math.min(deckIndex, decks.length - 1)]?.widget as WidgetDef | undefined);
  const keys = $derived((deck?.config?.buttons ?? []) as DeckButton[]);
  const key = $derived(keys[Math.min(picked, keys.length - 1)] as DeckButton | undefined);

  const looks: [KeyLook, string][] = [
    ['icon', t('Barevná ikona')],
    ['glyph', t('Jen symbol')],
    ['tinted', t('Tónované')],
    ['filled', t('Celé barevné')],
  ];

  const actionKinds: [Action['do'], string][] = [
    ['launch', t('Spustit program / web')],
    ['keys', t('Klávesová zkratka')],
    ['media.play_pause', t('Přehrát / pauza')],
    ['media.next', t('Další skladba')],
    ['media.prev', t('Předchozí skladba')],
    ['audio.mute_toggle', t('Ztlumit zvuk')],
    ['audio.set_volume', t('Nastavit hlasitost')],
    ['system.lock', t('Zamknout počítač')],
    ['display.off', t('Vypnout displej')],
    ['timer.toggle', t('Časovač: start / pauza')],
    ['timer.reset', t('Časovač: od začátku')],
    ['timer.skip', t('Časovač: přeskočit')],
  ];

  function deckConfig() {
    const d = deck!;
    if (!d.config) d.config = {};
    if (!d.config.buttons) d.config.buttons = [];
    return d.config;
  }

  function setKey<K extends keyof DeckButton>(field: K, value: DeckButton[K] | '' | undefined) {
    const k = key!;
    if (value === '' || value === undefined) delete k[field];
    else k[field] = value as DeckButton[K];
  }

  function setActionKind(kind: Action['do']) {
    const k = key!;
    if (kind === 'launch') k.action = { do: 'launch', target: '' };
    else if (kind === 'keys') k.action = { do: 'keys', combo: '' };
    else if (kind === 'audio.set_volume') k.action = { do: 'audio.set_volume', level: 0.5 };
    else k.action = { do: kind } as Action;
  }

  function addKey() {
    const list = deckConfig().buttons as DeckButton[];
    list.push({ label: t('Nové'), icon: 'star', color: 'blue', action: { do: 'media.play_pause' } });
    picked = list.length - 1;
  }

  function removeKey() {
    const list = deckConfig().buttons as DeckButton[];
    list.splice(picked, 1);
    picked = Math.max(0, picked - 1);
  }

  function moveKey(by: number) {
    const list = deckConfig().buttons as DeckButton[];
    const to = picked + by;
    if (to < 0 || to >= list.length) return;
    const [item] = list.splice(picked, 1);
    list.splice(to, 0, item);
    picked = to;
  }

  const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);
  const str = (e: Event) => (e.currentTarget as HTMLInputElement | HTMLSelectElement).value;
  const on = (e: Event) => (e.currentTarget as HTMLInputElement).checked;
</script>

{#snippet slider(label: string, value: number, min: number, max: number, step: number, change: (v: number) => void, unit = '')}
  <label class="row">
    <span>{label}</span>
    <input type="range" {min} {max} {step} {value} oninput={(e) => change(num(e))} />
    <output>{Math.round(value * 100) / 100}{unit}</output>
  </label>
{/snippet}

{#snippet swatches(value: string | undefined, change: (v: string) => void)}
  <div class="swatches">
    {#each Object.keys(palette) as name (name)}
      <button
        class="swatch"
        class:on={value === name}
        style="background:{palette[name]}"
        title={name}
        aria-label={name}
        onclick={() => change(name)}
      ></button>
    {/each}
    <input type="color" value={hex6(value ?? '#888888')} oninput={(e) => change(str(e))} title={t('Vlastní barva')} />
  </div>
{/snippet}

<div class="shell">
  <aside>
    <header>
      <h1>StreamPhone</h1>
      {#if cfg}
        <select class="language" title={t('Jazyk')} aria-label={t('Jazyk')} value={cfg.language ?? 'auto'} onchange={(e) => chooseLanguage(str(e))}>
          <option value="auto">{t('Podle zařízení')}</option>
          {#each Object.entries(LANGUAGES) as [code, name] (code)}
            <option value={code}>{name}</option>
          {/each}
        </select>
      {/if}
      <span class="state {status}">
        {#if status === 'saving'}{t('Ukládám…')}{:else if status === 'saved'}{t('Uloženo')}{:else if status === 'error'}{t('Chyba')}{/if}
      </span>
    </header>

    {#if status === 'error'}
      <p class="problem">{problem}</p>
    {/if}

    <nav>
      {#each tabs as [id, name] (id)}
        <button class:on={tab === id} onclick={() => (tab = id)}>{name}</button>
      {/each}
    </nav>

    {#if cfg}
      <div class="pane">
        {#if tab === 'layout' && page}
          <h2>{t('Rozložení')}</h2>
          <div class="chips">
            {#each cfg.pages as pg, i (pg.id)}
              <button
                class:on={i === Math.min(pageIndex, cfg.pages.length - 1)}
                onclick={() => {
                  pageIndex = i;
                  selectedId = null;
                }}>{pg.name}</button
              >
            {/each}
            <button class="plus" onclick={addPage} title={t('Přidat stránku')}>+</button>
          </div>
          <label class="row">
            <span>{t('Název stránky')}</span>
            <input class="wide" type="text" value={page.name} oninput={(e) => (page.name = str(e))} />
          </label>
          <div class="actions">
            <button class="btn" onclick={() => movePage(-1)} disabled={pageIndex === 0}>{t('← Dřív')}</button>
            <button class="btn" onclick={() => movePage(1)} disabled={pageIndex >= cfg.pages.length - 1}>{t('Později →')}</button>
            <button class="btn danger" onclick={removePage} disabled={cfg.pages.length < 2}>{t('Smazat stránku')}</button>
          </div>

          <h3>{t('Přidat widget')}</h3>
          <div class="actions">
            {#each Object.keys(widgetNames) as type (type)}
              <button class="btn" onclick={() => addWidget(type)}>+ {widgetNames[type]}</button>
            {/each}
          </div>
          <p class="hint">{@html t('Widgety přetahuj přímo v náhledu vpravo. Velikost změníš tažením za roh. Překrývající se widgety jsou červené.')}</p>

          {#if widget}
            <fieldset>
              <h3>{widgetNames[widget.type] ?? widget.type}</h3>
              {@render slider(t('Sloupec'), widget.x + 1, 1, 13 - widget.w, 1, (v) => (widget.x = v - 1))}
              {@render slider(t('Řádek'), widget.y + 1, 1, 7 - widget.h, 1, (v) => (widget.y = v - 1))}
              {@render slider(t('Šířka'), widget.w, 1, 12 - widget.x, 1, (v) => (widget.w = v))}
              {@render slider(t('Výška'), widget.h, 1, 6 - widget.y, 1, (v) => (widget.h = v))}

              {#if widget.type === 'clock'}
                <label class="row">
                  <span>{t('Formát')}</span>
                  <select value={widget.config?.format ?? '24h'} onchange={(e) => setOption('format', str(e), '24h')}>
                    <option value="24h">{t('24 hodin')}</option>
                    <option value="12h">{t('12 hodin')}</option>
                  </select>
                </label>
                <label class="row">
                  <span>{t('Vteřiny')}</span>
                  <input type="checkbox" checked={widget.config?.seconds !== false} onchange={(e) => setOption('seconds', on(e), true)} />
                </label>
                <label class="row">
                  <span>{t('Datum')}</span>
                  <input type="checkbox" checked={widget.config?.date !== false} onchange={(e) => setOption('date', on(e), true)} />
                </label>
                <label class="row">
                  <span>{t('Zarovnání')}</span>
                  <select value={widget.config?.align ?? ''} onchange={(e) => setOption('align', str(e), '')}>
                    <option value="">{t('Výchozí')}</option>
                    <option value="left">{t('Vlevo')}</option>
                    <option value="center">{t('Na střed')}</option>
                    <option value="right">{t('Vpravo')}</option>
                  </select>
                </label>
                {@render slider(t('Tloušťka'), widget.config?.weight ?? 500, 100, 900, 100, (v) => setOption('weight', v, 500))}
              {:else if widget.type === 'media'}
                <label class="row">
                  <span>{t('Obarvit obalem alba')}</span>
                  <input type="checkbox" checked={widget.config?.artBackground !== false} onchange={(e) => setOption('artBackground', on(e), true)} />
                </label>
              {:else if widget.type === 'system'}
                <label class="row">
                  <span>{t('Graf')}</span>
                  <input type="checkbox" checked={widget.config?.graph !== false} onchange={(e) => setOption('graph', on(e), true)} />
                </label>
              {:else if widget.type === 'ai'}
                <label class="row">
                  <span>{t('Nadpis')}</span>
                  <input class="wide" type="text" placeholder="Claude Code" value={widget.config?.title ?? ''} oninput={(e) => setOption('title', str(e), '')} />
                </label>
                <label class="row">
                  <span>{t('Povolit / Zamítnout')}</span>
                  <input type="checkbox" checked={widget.config?.approve !== false} onchange={(e) => setOption('approve', on(e), true)} />
                </label>
                {@render slider(t('Čekat na odpověď'), widget.config?.wait ?? 45, 5, 300, 5, (v) => setOption('wait', v, 45), ' s')}
                <h3>{t('Zapnutí v nástrojích')}</h3>
                <div class="chips">
                  {#each toolTabs as [id, name] (id)}
                    <button class:on={tool === id} onclick={() => (tool = id)}>{name}</button>
                  {/each}
                </div>
                {#if tool === 'claude'}
                  <p class="hint first">{@html t('Vlož do <code>%USERPROFILE%\\.claude\\settings.json</code> (sluč s tím, co tam už máš) a spusť Claude Code znovu.')}</p>
                  <textarea readonly rows="8" value={hookSnippet}></textarea>
                  <div class="actions">
                    <button class="btn" onclick={copyHooks}>{copied ? t('Zkopírováno') : t('Zkopírovat')}</button>
                  </div>
                {:else if tool === 'codex'}
                  <p class="hint first">{@html t('Plný přehled (začátek, nástroje, konec): ulož jako <code>%USERPROFILE%\\.codex\\hooks.json</code>. Codex se při prvním spuštění zeptá, jestli hookům důvěřuješ.')}</p>
                  <textarea readonly rows="8" value={codexSnippet}></textarea>
                  <p class="hint first" style="margin-top:10px">{@html t('Nebo jen oznámení o dokončení: přidej tento řádek do <code>%USERPROFILE%\\.codex\\config.toml</code>.')}</p>
                  <textarea readonly rows="2" value={codexNotify}></textarea>
                {:else}
                  <p class="hint first">{@html t('Cokoli dalšího se ohlásí příkazem agenta. Jméno za <code>--source</code> si zvol sám, ukáže se na telefonu jako štítek.')}</p>
                  <textarea readonly rows="8" value={otherSnippet}></textarea>
                {/if}
                <p class="hint">{@html t('S volbou Povolit / Zamítnout počká žádost o povolení nastavenou dobu na odpověď z telefonu. Když nepřijde, zeptá se Claude Code jako obvykle ve svém okně. Klepnutím na Povolit pouštíš příkaz na svém počítači, proto se na telefonu ukazuje celý.')}</p>
              {:else if widget.type === 'discord'}
                {@const ids = (widget.config?.servers as string[] | undefined) ?? []}
                <label class="row">
                  <span>{t('Nadpis')}</span>
                  <input class="wide" type="text" placeholder="Discord" value={widget.config?.title ?? ''} oninput={(e) => setOption('title', str(e), '')} />
                </label>
                <h3>{t('Servery')}</h3>
                {#each ids as id, i (i)}
                  <div class="row">
                    <input
                      class="wide"
                      type="text"
                      inputmode="numeric"
                      placeholder={t('ID serveru, např. 290926798626999250')}
                      value={id}
                      onchange={(e) => setOption('servers', ids.map((old, j) => (j === i ? str(e).trim() : old)), null)}
                    />
                    <button class="x" title={t('Odebrat')} onclick={() => setOption('servers', ids.filter((_, j) => j !== i), null)}>×</button>
                  </div>
                {/each}
                <div class="actions">
                  <button class="btn" onclick={() => setOption('servers', [...ids, ''], null)} disabled={ids.length >= 5}>+ Server</button>
                </div>
                <h3>{t('Přátelé')}</h3>
                <label class="row">
                  <span>{t('Jména')}</span>
                  <input
                    class="wide"
                    type="text"
                    placeholder="adam, Zuzka, …"
                    value={((widget.config?.friends as string[] | undefined) ?? []).join(', ')}
                    onchange={(e) =>
                      setOption(
                        'friends',
                        str(e)
                          .split(',')
                          .map((n) => n.trim())
                          .filter(Boolean),
                        null,
                      )}
                  />
                </label>
                <label class="row">
                  <span>{t('Jen přátelé')}</span>
                  <input type="checkbox" checked={widget.config?.onlyFriends === true} onchange={(e) => setOption('onlyFriends', on(e), false)} />
                </label>
                <label class="row">
                  <span>{t('Zavibrovat, když přijdou')}</span>
                  <input type="checkbox" checked={widget.config?.notify !== false} onchange={(e) => setOption('notify', on(e), true)} />
                </label>
                <p class="hint">{@html t('Discord cizím aplikacím seznam přátel nevydává. Widget proto čte veřejný „widget serveru“: správce ho zapne v <code>Nastavení serveru → Widget</code>, kde je i ID serveru. Pak je vidět, kdo je na serveru online, co hraje a v jakém hlasovém kanálu sedí. Jména přátel piš tak, jak je Discord na serveru ukazuje; budou nahoře a zvýrazněná.')}</p>
              {:else if widget.type === 'models'}
                <label class="row">
                  <span>{t('Nadpis')}</span>
                  <input class="wide" type="text" placeholder={t('Lokální modely')} value={widget.config?.title ?? ''} oninput={(e) => setOption('title', str(e), '')} />
                </label>
                <label class="row">
                  <span>{t('Skrýt vypnuté')}</span>
                  <input type="checkbox" checked={widget.config?.hideOffline === true} onchange={(e) => setOption('hideOffline', on(e), false)} />
                </label>
                <h3>{t('Sledované servery')}</h3>
                {#each knownServers as known (known.url)}
                  <label class="row">
                    <span>{known.name}</span>
                    <input type="checkbox" checked={serverList.some((s) => s.url === known.url)} onchange={(e) => toggleServer(known, on(e))} />
                  </label>
                {/each}
                {#each serverList as s, i (i)}
                  {#if !knownServers.some((k) => k.url === s.url)}
                    <div class="row">
                      <input class="wide" type="text" placeholder={t('Název')} value={s.name} onchange={(e) => editServer(i, 'name', str(e))} />
                      <input class="wide" style="margin-left:6px;flex:2" type="text" placeholder="http://127.0.0.1:8080" value={s.url} onchange={(e) => editServer(i, 'url', str(e))} />
                      <button class="x" title={t('Odebrat')} onclick={() => setServers(serverList.filter((_, j) => j !== i))}>×</button>
                    </div>
                  {/if}
                {/each}
                <div class="actions">
                  <button class="btn" onclick={() => setServers([...serverList, { name: '', url: '', kind: 'openai' }])}>{t('+ Další server')}</button>
                </div>
                <p class="hint">{@html t('Další server je cokoli s rozhraním kompatibilním s OpenAI: llama.cpp, Jan, KoboldCpp, LocalAI, vLLM, text-generation-webui. Zadej adresu bez <code>/v1</code>, jen <code>http://</code>.')}</p>
              {:else if widget.type === 'timer'}
                {@render slider(t('Soustředění'), widget.config?.focus ?? 25, 5, 90, 5, (v) => setOption('focus', v, 25), ' min')}
                {@render slider(t('Přestávka'), widget.config?.rest ?? 5, 1, 30, 1, (v) => setOption('rest', v, 5), ' min')}
              {:else if widget.type === 'todo'}
                <label class="row">
                  <span>{t('Nadpis')}</span>
                  <input class="wide" type="text" placeholder={t('Úkoly')} value={widget.config?.title ?? ''} oninput={(e) => setOption('title', str(e), '')} />
                </label>
              {:else if widget.type === 'processes'}
                <label class="row">
                  <span>{t('Řadit podle')}</span>
                  <select value={widget.config?.sort ?? 'cpu'} onchange={(e) => setOption('sort', str(e), 'cpu')}>
                    <option value="cpu">{t('Procesoru')}</option>
                    <option value="mem">{t('Paměti')}</option>
                  </select>
                </label>
                <label class="row">
                  <span>{t('Povolit ukončení')}</span>
                  <input type="checkbox" checked={widget.config?.kill !== false} onchange={(e) => setOption('kill', on(e), true)} />
                </label>
              {:else if widget.type === 'calendar'}
                <label class="row">
                  <span>{t('Týden začíná')}</span>
                  <select value={widget.config?.weekStart ?? 'monday'} onchange={(e) => setOption('weekStart', str(e), 'monday')}>
                    <option value="monday">{t('V pondělí')}</option>
                    <option value="sunday">{t('V neděli')}</option>
                  </select>
                </label>
              {:else if widget.type === 'deck'}
                <div class="actions">
                  <button class="btn" onclick={editKeys}>{t('Upravit tlačítka →')}</button>
                </div>
              {/if}

              <h3>{t('Vzhled widgetu')}</h3>
              <div class="row top">
                <span>{t('Zvýraznění')}</span>
                {@render swatches(widget.style?.accent, (v) => setStyle('accent', v))}
              </div>
              <div class="row top">
                <span>{t('Barva karty')}</span>
                {@render swatches(widget.style?.background, (v) => setStyle('background', v))}
              </div>
              {@render slider(t('Průhlednost'), 1 - (widget.style?.opacity ?? look.card.opacity), 0, 1, 0.05, (v) => setStyle('opacity', Math.round((1 - v) * 100) / 100))}
              <div class="actions">
                <button class="btn" onclick={() => delete widget.style} disabled={!widget.style}>{t('Vzhled podle motivu')}</button>
                <button class="btn danger" onclick={removeWidget}>{t('Smazat widget')}</button>
              </div>
            </fieldset>
          {/if}
        {:else if tab === 'theme'}
          <h2>{t('Motiv')}</h2>
          <div class="themes">
            {#each themeNames as name (name)}
              {@const sample = resolveTheme(name, own)}
              <div class="theme" class:on={currentPreset === name}>
                <button
                  class="sample"
                  style="background:{sample.background?.fill ?? sample.colors.bg}"
                  onclick={() => pickPreset(name)}
                  aria-label={name}
                >
                  <i style="background:{sample.colors.card};border-color:{sample.colors.line};border-radius:{sample.card.radius * 2}px">
                    <b style="background:{sample.colors.accent}"></b>
                    <u style="background:{sample.colors.text}"></u>
                    <u style="background:{sample.colors.dim};width:40%"></u>
                  </i>
                </button>
                <span>
                  {name}
                  {#if own[name]}<button class="x" title={t('Smazat motiv')} onclick={() => removeTheme(name)}>×</button>{/if}
                </span>
              </div>
            {/each}
          </div>
          <div class="actions">
            <button class="btn" onclick={saveAsTheme}>{t('Uložit jako vlastní motiv')}</button>
            <button class="btn danger" onclick={reset}>{t('Výchozí nastavení')}</button>
          </div>
          <p class="hint">{@html t('Vlastní motiv si zapamatuje barvy, karty, pozadí i písmo. Pak ho můžeš přiřadit jednotlivým stránkám.')}</p>
        {:else if tab === 'colors'}
          <h2>{t('Barvy')}</h2>
          {#each colorRows as [key, name] (key)}
            {@const custom = typeof cfg.theme === 'object' && cfg.theme?.colors?.[key] !== undefined}
            <label class="row">
              <span>{name}</span>
              <input type="color" value={hex6(look.colors[key])} oninput={(e) => setColor(key, str(e))} />
              <input class="code" type="text" value={look.colors[key]} onchange={(e) => setColor(key, str(e))} />
              <button class="x" disabled={!custom} title={t('Vrátit barvu motivu')} onclick={() => setColor(key, null)}>↺</button>
            </label>
          {/each}
        {:else if tab === 'cards'}
          <h2>{t('Karty')}</h2>
          {@render slider(t('Zaoblení'), look.card.radius, 0, 9, 0.2, (v) => setCard('radius', v))}
          {@render slider(t('Průhlednost'), 1 - look.card.opacity, 0, 1, 0.05, (v) => setCard('opacity', Math.round((1 - v) * 100) / 100))}
          {@render slider(t('Matné sklo'), look.card.blur, 0, 6, 0.5, (v) => setCard('blur', v))}
          {@render slider(t('Mezery'), look.gap, 0, 5, 0.25, (v) => (theme().gap = v))}
          <label class="row">
            <span>{t('Okraj')}</span>
            <input type="checkbox" checked={look.card.border} onchange={(e) => setCard('border', on(e))} />
          </label>
          <label class="row">
            <span>{t('Hloubka (lesk a stín)')}</span>
            <input type="checkbox" checked={look.card.depth} onchange={(e) => setCard('depth', on(e))} />
          </label>
          <label class="row">
            <span>{t('Tečky stránek')}</span>
            <input type="checkbox" checked={cfg.dots !== false} onchange={(e) => (cfg!.dots = on(e))} />
          </label>
          <p class="hint">{@html t('Matné sklo je vidět jen u průhledných karet nad tapetou.')}</p>
        {:else if tab === 'wall'}
          <h2>{t('Pozadí')}</h2>
          <div class="walls">
            <button class="wall none" class:on={!look.background?.image} onclick={() => setWall('image', undefined)}>{t('Žádné')}</button>
            {#each images as file (file.name)}
              <button
                class="wall"
                class:on={look.background?.image === file.name}
                style="background-image:url('{assetUrl(file.name)}')"
                title={file.name}
                aria-label={file.name}
                onclick={() => pickWall(file.name)}
              ></button>
            {/each}
            <label class="wall add">
              {t('+ Nahrát')}
              <input type="file" accept="image/*" onchange={(e) => upload(e, pickWall)} />
            </label>
          </div>
          <label class="row">
            <span>{t('Umístění')}</span>
            <select value={look.background?.fit ?? 'cover'} onchange={(e) => setWall('fit', str(e) as Background['fit'])}>
              <option value="cover">{t('Vyplnit')}</option>
              <option value="contain">{t('Přizpůsobit')}</option>
              <option value="stretch">{t('Roztáhnout')}</option>
              <option value="tile">{t('Dlaždice')}</option>
            </select>
          </label>
          {@render slider(t('Ztmavení'), look.background?.dim ?? 0, 0, 0.9, 0.05, (v) => setWall('dim', v))}
          {@render slider(t('Rozostření'), look.background?.blur ?? 0, 0, 8, 0.5, (v) => setWall('blur', v))}
          <label class="row">
            <span>{t('Barva / přechod')}</span>
            <input
              class="wide"
              type="text"
              placeholder="linear-gradient(135deg, #1b2a4a, #3b2a5c)"
              value={look.background?.fill ?? ''}
              onchange={(e) => setWall('fill', str(e))}
            />
          </label>
          <p class="hint">{@html t('Obrázky se ukládají do složky <code>assets</code> vedle nastavení. K tapetě se hodí průhledné karty (záložka Karty).')}</p>
        {:else if tab === 'font'}
          <h2>{t('Písmo')}</h2>
          <label class="row">
            <span>{t('Styl')}</span>
            <select value={look.font.family ?? ''} onchange={(e) => setFont('family', str(e))}>
              {#each fontChoices as [value, name] (value)}
                <option {value}>{name}</option>
              {/each}
              {#if look.font.family && !fontChoices.some(([v]) => v === look.font.family)}
                <option value={look.font.family}>{look.font.family}</option>
              {/if}
            </select>
          </label>
          <label class="row">
            <span>{t('Vlastní soubor')}</span>
            <select value={look.font.file ?? ''} onchange={(e) => setFont('file', str(e))}>
              <option value="">{t('Žádný')}</option>
              {#each fonts as file (file.name)}
                <option value={file.name}>{file.name}</option>
              {/each}
            </select>
            <label class="btn">
              {t('Nahrát')}
              <input type="file" accept=".ttf,.otf,.woff,.woff2" onchange={(e) => upload(e, (name) => setFont('file', name))} />
            </label>
          </label>
          <p class="hint">{@html t('Vlastní soubor (.ttf, .otf, .woff, .woff2) má přednost před stylem.')}</p>
        {:else if tab === 'pages'}
          <h2>{t('Stránky')}</h2>
          {#each cfg.pages as page (page.id)}
            <fieldset>
              <label class="row">
                <span>{t('Název')}</span>
                <input class="wide" type="text" value={page.name} onchange={(e) => (page.name = str(e))} />
              </label>
              <label class="row">
                <span>{t('Motiv')}</span>
                <select value={typeof page.theme === 'string' ? page.theme : ''} onchange={(e) => setPageTheme(page, str(e))}>
                  <option value="">{t('Jako ostatní')}</option>
                  {#each themeNames as name (name)}
                    <option value={name}>{name}</option>
                  {/each}
                </select>
              </label>
              <label class="row">
                <span>{t('Tapeta')}</span>
                <select value={pageWall(page)} onchange={(e) => setPageWall(page, str(e))}>
                  <option value="">{t('Jako ostatní')}</option>
                  <option value="-">{t('Žádná')}</option>
                  {#each images as file (file.name)}
                    <option value={file.name}>{file.name}</option>
                  {/each}
                </select>
              </label>
            </fieldset>
          {/each}
        {:else if tab === 'keys'}
          <h2>{t('Tlačítka')}</h2>
          {#if deck}
            <label class="row">
              <span>{t('Panel')}</span>
              <select
                value={String(Math.min(deckIndex, decks.length - 1))}
                onchange={(e) => {
                  deckIndex = Number(str(e));
                  picked = 0;
                }}
              >
                {#each decks as d, i (d.widget.id)}
                  <option value={String(i)}>{d.page.name} – {d.widget.id}</option>
                {/each}
              </select>
            </label>
            <label class="row">
              <span>{t('Vzhled')}</span>
              <select value={deck.config?.look ?? 'icon'} onchange={(e) => (deckConfig().look = str(e))}>
                {#each looks as [value, name] (value)}
                  <option {value}>{name}</option>
                {/each}
              </select>
            </label>
            {@render slider(t('Sloupce'), deck.config?.cols ?? 6, 1, 8, 1, (v) => (deckConfig().cols = v))}
            <label class="row">
              <span>{t('Popisky')}</span>
              <input type="checkbox" checked={deck.config?.labels !== false} onchange={(e) => (deckConfig().labels = on(e))} />
            </label>

            <div class="keys" style="grid-template-columns:repeat({deck.config?.cols ?? 6},1fr)">
              {#each keys as k, i (i)}
                {@const Icon = icons[k.icon ?? 'square'] ?? icons.square}
                <button class="keycap" class:on={i === picked} onclick={() => (picked = i)} title={k.label}>
                  {#if k.text}
                    <b>{k.text}</b>
                  {:else if k.image}
                    <img src={assetUrl(k.image)} alt="" />
                  {:else}
                    <span style="color:{palette[k.color ?? ''] ?? k.color ?? '#fff'}"><Icon size={18} /></span>
                  {/if}
                </button>
              {/each}
              <button class="keycap add" onclick={addKey} title={t('Přidat tlačítko')}>+</button>
            </div>

            {#if key}
              <fieldset>
                <div class="actions">
                  <button class="btn" onclick={() => moveKey(-1)} disabled={picked === 0}>{t('← Posunout')}</button>
                  <button class="btn" onclick={() => moveKey(1)} disabled={picked >= keys.length - 1}>{t('Posunout →')}</button>
                  <button class="btn danger" onclick={removeKey}>{t('Smazat')}</button>
                </div>
                <label class="row">
                  <span>{t('Popisek')}</span>
                  <input class="wide" type="text" value={key.label} oninput={(e) => (key.label = str(e))} />
                </label>
                <label class="row">
                  <span>{t('Velký text')}</span>
                  <input class="wide" type="text" placeholder={t('např. BRB nebo 1')} value={key.text ?? ''} oninput={(e) => setKey('text', str(e))} />
                </label>
                <label class="row">
                  <span>{t('Nadpisek')}</span>
                  <input class="wide" type="text" placeholder={t('např. SCENE')} value={key.caption ?? ''} oninput={(e) => setKey('caption', str(e))} />
                </label>
                <label class="row">
                  <span>{t('Vzhled')}</span>
                  <select value={key.look ?? ''} onchange={(e) => setKey('look', str(e) as KeyLook)}>
                    <option value="">{t('Podle panelu')}</option>
                    {#each looks as [value, name] (value)}
                      <option {value}>{name}</option>
                    {/each}
                  </select>
                </label>
                <div class="row top">
                  <span>{t('Barva')}</span>
                  {@render swatches(key.color, (v) => setKey('color', v))}
                </div>
                <label class="row">
                  <span>{t('Tečka')}</span>
                  <select value={key.dot ?? ''} onchange={(e) => setKey('dot', str(e))}>
                    <option value="">{t('Žádná')}</option>
                    {#each Object.keys(palette) as name (name)}
                      <option value={name}>{name}</option>
                    {/each}
                  </select>
                </label>
                <label class="row">
                  <span>{t('Obrázek')}</span>
                  <select value={key.image ?? ''} onchange={(e) => setKey('image', str(e))}>
                    <option value="">{t('Žádný (ikona)')}</option>
                    {#each images as file (file.name)}
                      <option value={file.name}>{file.name}</option>
                    {/each}
                  </select>
                  <label class="btn">
                    {t('Nahrát')}
                    <input type="file" accept="image/*" onchange={(e) => upload(e, (name) => setKey('image', name))} />
                  </label>
                </label>
                <div class="row top">
                  <span>{t('Ikona')}</span>
                  <div class="icons">
                    {#each iconNames as name (name)}
                      {@const Icon = icons[name]}
                      <button class:on={(key.icon ?? 'square') === name} title={name} aria-label={name} onclick={() => setKey('icon', name)}>
                        <Icon size={18} />
                      </button>
                    {/each}
                  </div>
                </div>

                <h3>{t('Co tlačítko udělá')}</h3>
                <label class="row">
                  <span>{t('Akce')}</span>
                  <select value={key.action.do} onchange={(e) => setActionKind(str(e) as Action['do'])}>
                    {#each actionKinds as [value, name] (value)}
                      <option {value}>{name}</option>
                    {/each}
                  </select>
                </label>
                {#if key.action.do === 'launch'}
                  {@const a = key.action}
                  <label class="row">
                    <span>{t('Cíl')}</span>
                    <input class="wide" type="text" placeholder={t('program, soubor, https://… nebo spotify:')} value={a.target} onchange={(e) => (a.target = str(e))} />
                  </label>
                  <label class="row">
                    <span>{t('Parametry')}</span>
                    <input
                      class="wide"
                      type="text"
                      value={a.args ?? ''}
                      onchange={(e) => {
                        if (str(e)) a.args = str(e);
                        else delete a.args;
                      }}
                    />
                  </label>
                {:else if key.action.do === 'keys'}
                  {@const a = key.action}
                  <label class="row">
                    <span>{t('Zkratka')}</span>
                    <input class="wide" type="text" placeholder="ctrl+shift+esc" value={a.combo} onchange={(e) => (a.combo = str(e))} />
                  </label>
                {:else if key.action.do === 'audio.set_volume'}
                  {@const a = key.action}
                  {@render slider(t('Hlasitost'), Math.round(a.level * 100), 0, 100, 5, (v) => (a.level = v / 100), ' %')}
                {/if}
              </fieldset>
            {/if}
          {:else}
            <p class="hint">{@html t('V nastavení není žádný panel tlačítek.')}</p>
          {/if}
        {:else if tab === 'link'}
          <h2>{t('Připojení')}</h2>
          <h3>{t('Kabelem')}</h3>
          <p class="hint first">{@html t('Zapni na telefonu <code>Možnosti pro vývojáře → Ladění USB</code> a připoj ho. Dashboard se otevře sám, párovat není třeba.')}</p>

          <h3>{t('Přes Wi-Fi')}</h3>
          <label class="row">
            <span>{t('Povolit Wi-Fi')}</span>
            <input type="checkbox" checked={cfg.wireless === true} onchange={(e) => (cfg!.wireless = on(e))} />
          </label>
          {#if cfg.wireless}
            {#if pairing?.address && pairing.qr}
              <div class="qr">{@html pairing.qr}</div>
              <p class="hint center">{@html t('Naskenuj fotoaparátem telefonu. Telefon musí být ve stejné síti jako počítač.')}</p>
              <label class="row">
                <span>{t('Adresa')}</span>
                <input class="wide code" style="margin-left:0" type="text" readonly value={pairing.url ?? ''} />
              </label>
              <div class="actions">
                <button class="btn" onclick={() => loadLink(true)}>{t('Nový kód')}</button>
                <span class="state" style="align-self:center;margin-left:12px">{t('platí ještě {n} min, na jedno použití', { n: Math.ceil(pairing.seconds / 60) })}</span>
              </div>
            {:else if pairing && !pairing.address}
              <p class="problem" style="margin:12px 0 0">{@html t('Počítač není v místní síti (nemá soukromou adresu), takže se k němu telefon přes Wi-Fi nedostane.')}</p>
            {:else}
              <p class="hint">{@html t('Připravuji kód…')}</p>
            {/if}

            <h3>{t('Spárované telefony')}</h3>
            {#each paired as device (device.id)}
              <div class="row">
                <span style="flex:1">{device.name}</span>
                <output style="flex:none">{new Date(device.created).toLocaleDateString(language())}</output>
                <button class="x" title={t('Zrušit spárování')} onclick={() => forget(device)}>×</button>
              </div>
            {:else}
              <p class="hint first">{@html t('Zatím žádný.')}</p>
            {/each}

            <p class="hint">{@html t('Windows se při prvním zapnutí zeptá, jestli má StreamPhone povolit v síti: povol ho pro <strong>soukromé</strong> sítě. Bez spárování se k tlačítkům nikdo ze sítě nedostane. Spojení ale není šifrované, takže Wi-Fi používej jen v síti, které věříš (doma), ne na veřejné.')}</p>
          {/if}
        {:else if tab === 'night'}
          <h2>{t('Noční režim')}</h2>
          <label class="row">
            <span>{t('Zapnuto')}</span>
            <input type="checkbox" checked={cfg.night?.enabled === true} onchange={(e) => setNight('enabled', on(e))} />
          </label>
          <label class="row">
            <span>{t('Od')}</span>
            <input class="wide" type="time" value={cfg.night?.from ?? '22:00'} onchange={(e) => setNight('from', str(e))} />
          </label>
          <label class="row">
            <span>{t('Do')}</span>
            <input class="wide" type="time" value={cfg.night?.to ?? '07:00'} onchange={(e) => setNight('to', str(e))} />
          </label>
          {@render slider(t('Ztmavení'), cfg.night?.dim ?? 0.5, 0, 0.95, 0.05, (v) => setNight('dim', v))}
          <label class="row">
            <span>{t('Jen červené světlo')}</span>
            <input type="checkbox" checked={cfg.night?.red === true} onchange={(e) => setNight('red', on(e))} />
          </label>
          <label class="row">
            <span>{t('Přepnout na stránku')}</span>
            <select value={cfg.night?.page ?? ''} onchange={(e) => setNight('page', str(e))}>
              <option value="">{t('Nepřepínat')}</option>
              {#each cfg.pages as pg (pg.id)}
                <option value={pg.id}>{pg.name}</option>
              {/each}
            </select>
          </label>
          <p class="hint">{@html t('V nastaveném čase dashboard ztmavne, třeba pro telefon na nočním stolku. Chceš-li si ho hned prohlédnout, nastav rozsah tak, aby zahrnoval aktuální čas.')}</p>
        {/if}
      </div>
    {/if}
  </aside>

  <main>
    {#if cfg}
      <div class="phone" bind:clientWidth={phoneW} bind:clientHeight={phoneH}>
        <iframe title={t('Náhled telefonu')} src="http://127.0.0.1:{dashboardPort ?? cfg.port}/" bind:this={frame} onload={() => tab === 'layout' && showPage()}></iframe>
        {#if tab === 'layout' && page && phoneH}
          <Layout
            {page}
            gap={look.gap}
            width={phoneW}
            height={phoneH}
            selected={selectedId}
            names={widgetNames}
            onselect={(id) => (selectedId = id)}
            onchange={place}
          />
        {/if}
      </div>
      <p class="hint center">{@html t('Živý náhled. Změny se ukládají samy a na připojeném telefonu se projeví hned.')}</p>
    {/if}
  </main>
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: #0b0b0d;
    color: #f5f5f7;
    font: 14px/1.4 -apple-system, 'SF Pro Text', Inter, system-ui, 'Segoe UI', Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  :global(#app) {
    height: 100%;
  }
  :global(*) {
    box-sizing: border-box;
  }
  .shell {
    display: flex;
    height: 100%;
  }
  aside {
    width: 420px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: #161618;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    min-height: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 20px 22px 8px;
  }
  .language {
    flex: 0 1 150px;
    margin: 0 10px 0 auto;
    padding: 4px 6px;
    font-size: 12px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  h2 {
    margin: 0 0 14px;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  h3 {
    margin: 18px 0 8px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #8e8e93;
  }
  .state {
    font-size: 12px;
    color: #8e8e93;
  }
  .state.saved {
    color: #30d158;
  }
  .state.error {
    color: #ff453a;
  }
  .problem {
    margin: 0 22px 8px;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(255, 69, 58, 0.14);
    color: #ff8a82;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    padding: 8px 18px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  nav button {
    margin: 3px;
    padding: 6px 12px;
    border-radius: 999px;
    color: #c7c7cc;
    font-weight: 600;
  }
  nav button.on {
    background: #0a84ff;
    color: #fff;
  }
  .pane {
    padding: 20px 22px 40px;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  button {
    font: inherit;
    color: inherit;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .row {
    display: flex;
    align-items: center;
    min-height: 40px;
    padding: 4px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .row.top {
    align-items: flex-start;
    padding: 10px 0;
  }
  .row > span:first-child {
    flex: 0 0 132px;
    color: #c7c7cc;
  }
  .row output {
    flex: 0 0 52px;
    text-align: right;
    color: #8e8e93;
    font-variant-numeric: tabular-nums;
  }
  input[type='range'] {
    flex: 1;
    min-width: 0;
    accent-color: #0a84ff;
  }
  input[type='checkbox'] {
    width: 18px;
    height: 18px;
    margin-left: auto;
    accent-color: #0a84ff;
  }
  input[type='text'],
  input[type='time'],
  select {
    font: inherit;
    color: inherit;
    background: #2c2c2e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 6px 9px;
    min-width: 0;
    flex: 1;
  }
  input[type='color'] {
    width: 34px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: none;
    flex: none;
    cursor: pointer;
  }
  .code {
    margin-left: 8px;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 12px;
  }
  .x {
    width: 26px;
    height: 26px;
    margin-left: 6px;
    border-radius: 50%;
    color: #8e8e93;
    flex: none;
  }
  .x:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
  input[type='file'] {
    display: none;
  }
  .btn {
    display: inline-block;
    padding: 7px 13px;
    margin-left: 8px;
    border-radius: 9px;
    background: #2c2c2e;
    font-weight: 600;
    cursor: pointer;
    flex: none;
  }
  .btn:hover:not(:disabled) {
    background: #3a3a3c;
  }
  .btn.danger {
    color: #ff453a;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    margin: 12px 0 4px -8px;
  }
  .hint {
    color: #8e8e93;
    font-size: 12.5px;
    margin: 14px 0 0;
  }
  .hint.first {
    margin: 0 0 10px;
  }
  textarea {
    width: 100%;
    font: 11.5px/1.45 ui-monospace, Consolas, monospace;
    color: #c7c7cc;
    background: #1c1c1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 8px 10px;
    resize: vertical;
  }
  .hint.center {
    text-align: center;
  }
  .pane :global(code) {
    font-family: ui-monospace, Consolas, monospace;
    color: #c7c7cc;
  }
  fieldset {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    margin: 14px 0 0;
    padding: 6px 14px 12px;
    min-width: 0;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    margin: 0 -3px 10px;
  }
  .chips button {
    margin: 3px;
    padding: 6px 12px;
    border-radius: 9px;
    background: #2c2c2e;
    font-weight: 600;
    color: #c7c7cc;
  }
  .chips button.on {
    background: #f5f5f7;
    color: #000;
  }
  .chips .plus {
    background: none;
    border: 1px dashed rgba(255, 255, 255, 0.25);
  }
  fieldset h3:first-child {
    margin-top: 10px;
  }

  .qr {
    width: 220px;
    margin: 14px auto 0;
    padding: 12px;
    border-radius: 16px;
    background: #fff;
    line-height: 0;
  }
  .qr :global(svg) {
    width: 100%;
    height: auto;
  }

  /* Theme tiles */
  .themes {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-gap: 12px;
  }
  .theme {
    text-align: center;
    font-size: 12.5px;
    color: #c7c7cc;
  }
  .theme > span {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 26px;
  }
  .sample {
    display: block;
    width: 100%;
    height: 70px;
    padding: 10px;
    border-radius: 12px;
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.1);
  }
  .theme.on .sample {
    border-color: #0a84ff;
  }
  .sample i {
    display: block;
    height: 100%;
    padding: 8px;
    border: 1px solid;
  }
  .sample b,
  .sample u {
    display: block;
    height: 5px;
    border-radius: 3px;
    margin-bottom: 5px;
    width: 70%;
  }
  .sample b {
    width: 30%;
    height: 8px;
  }

  /* Wallpapers */
  .walls {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-gap: 10px;
    margin-bottom: 12px;
  }
  .wall {
    height: 62px;
    border-radius: 10px;
    background: #2c2c2e center / cover no-repeat;
    border: 2px solid transparent;
    display: grid;
    place-items: center;
    color: #c7c7cc;
    font-weight: 600;
    cursor: pointer;
  }
  .wall.on {
    border-color: #0a84ff;
  }
  .wall.add {
    border: 1px dashed rgba(255, 255, 255, 0.25);
    background: none;
  }

  /* Deck keys */
  .keys {
    display: grid;
    grid-gap: 6px;
    margin: 14px 0 4px;
  }
  .keycap {
    height: 44px;
    border-radius: 10px;
    background: #2c2c2e;
    border: 2px solid transparent;
    display: grid;
    place-items: center;
    overflow: hidden;
    font-size: 11px;
  }
  .keycap.on {
    border-color: #0a84ff;
  }
  .keycap.add {
    background: none;
    border: 1px dashed rgba(255, 255, 255, 0.25);
    font-size: 18px;
    color: #8e8e93;
  }
  .keycap img {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    object-fit: cover;
  }
  .keycap span {
    display: grid;
  }
  .swatches,
  .icons {
    display: flex;
    flex-wrap: wrap;
    flex: 1;
    min-width: 0;
  }
  .swatch {
    width: 22px;
    height: 22px;
    margin: 0 6px 6px 0;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.2);
  }
  .swatch.on {
    box-shadow: 0 0 0 2px #161618, 0 0 0 4px #0a84ff;
  }
  .icons button {
    width: 30px;
    height: 30px;
    margin: 0 3px 3px 0;
    border-radius: 7px;
    display: grid;
    place-items: center;
    color: #c7c7cc;
  }
  .icons button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .icons button.on {
    background: #0a84ff;
    color: #fff;
  }

  /* Preview */
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: radial-gradient(circle at 50% 35%, #1b1b20, #0b0b0d 70%);
  }
  .phone {
    width: min(100%, 900px);
    /* The frame keeps a phone's proportions (19.5 : 9, lying on its side). */
    position: relative;
    padding-top: min(46.2%, 415px);
    border-radius: 34px;
    background: #000;
    box-shadow: 0 0 0 10px #1c1c1e, 0 0 0 11px rgba(255, 255, 255, 0.12), 0 30px 80px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }
  iframe {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    border: 0;
  }
</style>
