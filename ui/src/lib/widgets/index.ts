import type { Component } from 'svelte';
import type { WidgetDef } from '../types';
import Ai from './Ai.svelte';
import Calendar from './Calendar.svelte';
import Clock from './Clock.svelte';
import Deck from './Deck.svelte';
import Discord from './Discord.svelte';
import Media from './Media.svelte';
import Models from './Models.svelte';
import Processes from './Processes.svelte';
import System from './System.svelte';
import Timer from './Timer.svelte';
import Todo from './Todo.svelte';

/** Widget registry: `type` in config.json → component. */
export const widgets: Record<string, Component<{ def: WidgetDef }>> = {
  ai: Ai,
  calendar: Calendar,
  clock: Clock,
  deck: Deck,
  discord: Discord,
  media: Media,
  models: Models,
  processes: Processes,
  system: System,
  timer: Timer,
  todo: Todo,
};
