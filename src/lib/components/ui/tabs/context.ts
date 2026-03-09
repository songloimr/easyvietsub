import { getContext, setContext } from 'svelte';
import type { Readable } from 'svelte/store';

const TABS_CONTEXT_KEY = Symbol('tabs');

export interface TabsContextValue {
  baseId: string;
  value: Readable<string>;
  setValue: (next: string) => void;
}

export function setTabsContext(context: TabsContextValue): TabsContextValue {
  setContext(TABS_CONTEXT_KEY, context);
  return context;
}

export function getTabsContext(): TabsContextValue {
  return getContext<TabsContextValue>(TABS_CONTEXT_KEY);
}
