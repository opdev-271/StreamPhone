// Languages.
//
// The texts in the code are Czech, the language the app was first written in,
// and double as the lookup keys: `t('Hotovo')`. Each table row lists the other
// languages in the order of `COLUMNS`. A missing translation falls back to
// English, then to the Czech original, so a new text is never blank.

export const LANGUAGES = {
  en: 'English',
  cs: 'Čeština',
  sk: 'Slovenčina',
  de: 'Deutsch',
  pl: 'Polski',
  es: 'Español',
  fr: 'Français',
} as const;

export type Language = keyof typeof LANGUAGES;

/** The order of the translations in a table row. */
const COLUMNS: Language[] = ['en', 'sk', 'de', 'pl', 'es', 'fr'];

export type Table = Record<string, string[]>;

function supported(code: string | undefined | null): Language | null {
  const short = (code ?? '').toLowerCase().split(/[-_]/)[0];
  return short in LANGUAGES ? (short as Language) : null;
}

/** The first of the browser's languages we have, else English. */
function detect(): Language {
  const wanted = typeof navigator === 'undefined' ? [] : (navigator.languages ?? [navigator.language]);
  for (const code of wanted) {
    const found = supported(code);
    if (found) return found;
  }
  return 'en';
}

const state = $state({ language: detect() });

/** `setting` is the config's `language`: a code, or "auto" / nothing for the device's own. */
export function setLanguage(setting: string | undefined | null) {
  state.language = supported(setting) ?? detect();
  if (typeof document !== 'undefined') document.documentElement.lang = state.language;
}

export function language(): Language {
  return state.language;
}

/**
 * Makes the `t` of one table. `{name}` in a text is replaced from `values`.
 * Reading the language inside makes every use in a template update by itself
 * when the language changes.
 */
export function translator(table: Table) {
  return (czech: string, values?: Record<string, string | number>): string => {
    const lang = state.language;
    const row = table[czech];
    let text = lang === 'cs' ? czech : (row?.[COLUMNS.indexOf(lang)] ?? row?.[0] ?? czech);
    if (values) for (const name in values) text = text.split(`{${name}}`).join(String(values[name]));
    return text;
  };
}

/** A number the way the language writes it: 1,5 in Czech, 1.5 in English. */
export function number(value: number, digits = 0): string {
  return value.toLocaleString(state.language, { minimumFractionDigits: digits, maximumFractionDigits: digits });
}

/** A date in the language, e.g. `date(now, { weekday: 'long' })`. */
export function date(value: Date, options: Intl.DateTimeFormatOptions): string {
  return value.toLocaleDateString(state.language, options);
}
