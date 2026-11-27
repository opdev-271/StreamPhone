// Ready-made pages offered when a page is added. A template is only a start:
// once added it is an ordinary page and everything on it can be changed.
//
// Shortcuts are the programs' own defaults on Windows. Streaming programs have
// none, so the stream page presses F13–F24, which no keyboard has and which
// are therefore free to assign there.

import type { Action, DeckButton, WidgetDef } from '../lib/types';
import { t } from './strings';

export interface Template {
  id: string;
  name: string;
  about: string;
  /** Widgets without ids; the editor gives them ids when the page is added. */
  widgets: Omit<WidgetDef, 'id'>[];
}

type Box = [x: number, y: number, w: number, h: number];

const keys = (combo: string): Action => ({ do: 'keys', combo });
const launch = (target: string): Action => ({ do: 'launch', target });

/** A key with an icon. */
function key(label: string, icon: string, color: string, action: Action): DeckButton {
  return { label, icon, color, action };
}

/** A key with big text instead of an icon ("J", "IN", "LIVE"). */
function word(label: string, text: string, color: string, action: Action, more: Partial<DeckButton> = {}): DeckButton {
  return { label, text, color, action, look: 'tinted', ...more };
}

function widget(type: string, [x, y, w, h]: Box, config?: Record<string, unknown>): Omit<WidgetDef, 'id'> {
  return config ? { type, x, y, w, h, config } : { type, x, y, w, h };
}

function deck(box: Box, cols: number, buttons: DeckButton[]) {
  return widget('deck', box, { cols, buttons });
}

const FULL: Box = [0, 0, 12, 6];

export function templates(): Template[] {
  const undo = key(t('Zpět'), 'undo', 'gray', keys('ctrl+z'));
  const redo = key(t('Znovu'), 'redo', 'gray', keys('ctrl+shift+z'));
  const save = key(t('Uložit'), 'save', 'green', keys('ctrl+s'));
  // The shuttle keys every editing program shares.
  const transport = [
    key(t('Přehrát'), 'play', 'green', keys('space')),
    word(t('Dozadu'), 'J', 'blue', keys('j')),
    word(t('Pauza'), 'K', 'blue', keys('k')),
    word(t('Dopředu'), 'L', 'blue', keys('l')),
    word(t('Značka In'), 'IN', 'yellow', keys('i')),
    word(t('Značka Out'), 'OUT', 'yellow', keys('o')),
  ];

  return [
    {
      id: 'premiere',
      name: t('Střih videa · Premiere Pro'),
      about: t('Přehrávání, střih, značky a export na jedno klepnutí.'),
      widgets: [
        deck(FULL, 6, [
          ...transport,
          key(t('Střih'), 'scissors', 'red', keys('ctrl+k')),
          key(t('Smazat úsek'), 'trash', 'red', keys('shift+delete')),
          word(t('Ořez začátku'), 'Q', 'orange', keys('q')),
          word(t('Ořez konce'), 'W', 'orange', keys('w')),
          key(t('Značka'), 'flag', 'purple', keys('m')),
          key(t('Přichytávání'), 'magnet', 'purple', keys('s')),
          undo,
          redo,
          key(t('Přiblížit'), 'zoom-in', 'cyan', keys('=')),
          key(t('Oddálit'), 'zoom-out', 'cyan', keys('-')),
          save,
          key(t('Export'), 'upload', 'pink', keys('ctrl+m')),
        ]),
      ],
    },
    {
      id: 'resolve',
      name: t('Střih videa · DaVinci Resolve'),
      about: t('Přehrávání, střih, značky a export na jedno klepnutí.'),
      widgets: [
        deck(FULL, 6, [
          ...transport,
          key(t('Střih'), 'scissors', 'red', keys('ctrl+b')),
          key(t('Smazat úsek'), 'trash', 'red', keys('shift+backspace')),
          key(t('Ořez začátku'), 'prev', 'orange', keys('ctrl+shift+[')),
          key(t('Ořez konce'), 'next', 'orange', keys('ctrl+shift+]')),
          key(t('Značka'), 'flag', 'purple', keys('m')),
          key(t('Přichytávání'), 'magnet', 'purple', keys('n')),
          undo,
          redo,
          key(t('Přiblížit'), 'zoom-in', 'cyan', keys('ctrl+=')),
          key(t('Oddálit'), 'zoom-out', 'cyan', keys('ctrl+-')),
          key(t('Celá časová osa'), 'monitor', 'cyan', keys('shift+z')),
          save,
        ]),
      ],
    },
    {
      id: 'aftereffects',
      name: 'After Effects',
      about: t('Náhled, vrstvy, klíčové snímky a render.'),
      widgets: [
        deck(FULL, 6, [
          key(t('Náhled'), 'play', 'green', keys('space')),
          key(t('Předchozí snímek'), 'prev', 'blue', keys('pageup')),
          key(t('Další snímek'), 'next', 'blue', keys('pagedown')),
          word(t('Začátek oblasti'), 'B', 'yellow', keys('b')),
          word(t('Konec oblasti'), 'N', 'yellow', keys('n')),
          word(t('Klíčové snímky'), 'U', 'purple', keys('u')),
          key(t('Rozdělit vrstvu'), 'scissors', 'red', keys('ctrl+shift+d')),
          key(t('Duplikovat'), 'copy', 'orange', keys('ctrl+d')),
          key(t('Předkompozice'), 'layers', 'orange', keys('ctrl+shift+c')),
          key(t('Nová plocha'), 'square', 'indigo', keys('ctrl+y')),
          key(t('Vrstva úprav'), 'sun', 'indigo', keys('ctrl+alt+y')),
          word('Easy Ease', 'F9', 'purple', keys('f9')),
          undo,
          redo,
          key(t('Přizpůsobit oknu'), 'monitor', 'cyan', keys('shift+/')),
          save,
          key(t('Fronta renderu'), 'film', 'pink', keys('ctrl+m')),
          key('Media Encoder', 'upload', 'pink', keys('ctrl+alt+m')),
        ]),
      ],
    },
    {
      id: 'stream',
      name: t('Stream'),
      about: t('Vysílání, scény a mikrofon. V OBS nebo Streamlabs přiřaď akcím klávesy F13–F24.'),
      widgets: [
        deck([0, 0, 8, 6], 4, [
          word(t('Vysílání'), 'LIVE', 'red', keys('f13'), { look: 'filled', dot: 'white' }),
          word(t('Nahrávání'), 'REC', 'red', keys('f14'), { dot: 'red' }),
          word(t('Hned jsem zpět'), 'BRB', 'orange', keys('f15')),
          key(t('Mikrofon'), 'mic-off', 'yellow', keys('f16')),
          ...[1, 2, 3, 4].map((n) => word(t('Scéna {n}', { n }), String(n), 'blue', keys(`f${16 + n}`), { caption: 'SCENE' })),
          key(t('Kamera'), 'video', 'purple', keys('f21')),
          key(t('Ztlumit zvuk'), 'mute', 'gray', keys('f22')),
          key(t('Snímek obrazovky'), 'camera', 'cyan', keys('f23')),
          key(t('Uložit záznam'), 'download', 'green', keys('f24')),
        ]),
        widget('system', [8, 0, 4, 3]),
        widget('clock', [8, 3, 4, 3]),
      ],
    },
    {
      id: 'code',
      name: t('Programování'),
      about: t('Nástroje po ruce, AI relace a lokální modely na očích.'),
      widgets: [
        deck([0, 0, 4, 6], 2, [
          key(t('Terminál'), 'terminal', 'graphite', launch('wt.exe')),
          key('VS Code', 'code', 'blue', launch('%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe')),
          key(t('Prohlížeč'), 'globe', 'cyan', launch('https://www.google.com')),
          key(t('Soubory'), 'folder', 'yellow', launch('explorer.exe')),
          key(t('Výstřižek'), 'scissors', 'purple', launch('ms-screenclip:')),
          key(t('Zamknout'), 'lock', 'red', { do: 'system.lock' }),
        ]),
        widget('ai', [4, 0, 4, 6]),
        widget('models', [8, 0, 4, 3]),
        widget('processes', [8, 3, 4, 3]),
      ],
    },
    {
      id: 'meeting',
      name: t('Schůzky · Teams'),
      about: t('Mikrofon, kamera a sdílení obrazovky bez hledání v okně.'),
      widgets: [
        deck([0, 0, 8, 6], 3, [
          key(t('Mikrofon'), 'mic-off', 'red', keys('ctrl+shift+m')),
          key(t('Kamera'), 'video', 'blue', keys('ctrl+shift+o')),
          key(t('Zvednout ruku'), 'hand', 'yellow', keys('ctrl+shift+k')),
          key(t('Sdílet obrazovku'), 'monitor', 'green', keys('ctrl+shift+e')),
          key(t('Rozmazat pozadí'), 'image', 'purple', keys('ctrl+shift+p')),
          key(t('Odejít'), 'power', 'red', keys('ctrl+shift+h')),
        ]),
        widget('clock', [8, 0, 4, 3]),
        widget('calendar', [8, 3, 4, 3]),
      ],
    },
    {
      id: 'music',
      name: t('Hudba'),
      about: t('Co právě hraje, ovládání přehrávání a hlasitost.'),
      widgets: [
        widget('media', [0, 0, 8, 4]),
        widget('clock', [8, 0, 4, 4]),
        deck([0, 4, 12, 2], 6, [
          key(t('Předchozí'), 'prev', 'blue', { do: 'media.prev' }),
          key(t('Přehrát'), 'play', 'green', { do: 'media.play_pause' }),
          key(t('Další'), 'next', 'blue', { do: 'media.next' }),
          key(t('Tišeji'), 'volume', 'gray', keys('volumedown')),
          key(t('Hlasitěji'), 'volume', 'cyan', keys('volumeup')),
          key(t('Ztlumit'), 'mute', 'orange', { do: 'audio.mute_toggle' }),
        ]),
      ],
    },
    {
      id: 'game',
      name: t('Hraní'),
      about: t('Kdo je na Discordu, vytížení počítače a záznam hry.'),
      widgets: [
        widget('discord', [0, 0, 4, 6]),
        widget('system', [4, 0, 4, 3]),
        widget('media', [4, 3, 4, 3]),
        deck([8, 0, 4, 6], 2, [
          key(t('Mikrofon'), 'mic-off', 'red', keys('ctrl+shift+m')),
          key(t('Sluchátka'), 'headphones', 'orange', keys('ctrl+shift+d')),
          key(t('Snímek obrazovky'), 'camera', 'cyan', keys('win+alt+printscreen')),
          key(t('Nahrávat'), 'live', 'red', keys('win+alt+r')),
          key(t('Posledních 30 s'), 'download', 'green', keys('win+alt+g')),
          key(t('Ztlumit'), 'mute', 'gray', { do: 'audio.mute_toggle' }),
        ]),
      ],
    },
    {
      id: 'monitor',
      name: t('Přehled počítače'),
      about: t('Procesor, paměť, síť a nejnáročnější programy.'),
      widgets: [widget('system', [0, 0, 4, 6]), widget('processes', [4, 0, 8, 6])],
    },
  ];
}
