<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { getTabsContext } from './context';

  interface Props {
    value: string;
    class?: string;
    disabled?: boolean;
    children?: Snippet;
  }

  let { value, class: className = '', disabled = false, children }: Props = $props();

  const { baseId, value: valueStore, setValue } = getTabsContext();
  let element: HTMLButtonElement | null = null;

  function handleSelect(): void {
    if (!disabled) {
      setValue(value);
    }
  }

  function getEnabledTriggers(): HTMLButtonElement[] {
    const list = element?.closest('[role="tablist"]');
    if (!list) {
      return [];
    }

    return Array.from(list.querySelectorAll<HTMLButtonElement>('[role="tab"]:not([disabled])'));
  }

  function focusSibling(index: number): void {
    const target = getEnabledTriggers()[index];
    if (!target) {
      return;
    }

    target.focus();
    target.click();
  }
</script>

<button
  bind:this={element}
  class={cn(
    'inline-flex h-8 min-w-0 flex-1 items-center justify-center gap-2 whitespace-nowrap rounded-md border border-transparent px-2.5 py-1 text-sm font-medium transition-[color,box-shadow,background-color] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:border-border/80 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm',
    className
  )}
  id={`${baseId}-trigger-${value}`}
  data-slot="tabs-trigger"
  data-state={$valueStore === value ? 'active' : 'inactive'}
  role="tab"
  type="button"
  tabindex={$valueStore === value ? 0 : -1}
  aria-controls={`${baseId}-content-${value}`}
  aria-selected={$valueStore === value}
  aria-disabled={disabled}
  disabled={disabled}
  onclick={handleSelect}
  onkeydown={(event) => {
    const triggers = getEnabledTriggers();
    const currentIndex = triggers.findIndex((trigger) => trigger === element);

    if (event.key === 'ArrowRight' && currentIndex !== -1) {
      event.preventDefault();
      focusSibling((currentIndex + 1) % triggers.length);
      return;
    }

    if (event.key === 'ArrowLeft' && currentIndex !== -1) {
      event.preventDefault();
      focusSibling((currentIndex - 1 + triggers.length) % triggers.length);
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      focusSibling(0);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      focusSibling(triggers.length - 1);
      return;
    }

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleSelect();
    }
  }}
>
  {@render children?.()}
</button>
