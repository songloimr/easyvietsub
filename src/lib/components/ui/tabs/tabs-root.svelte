<script lang="ts" module>
  let tabsRootCount = 0;
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { toStore } from 'svelte/store';
  import { cn } from '$lib/utils';
  import { setTabsContext } from './context';

  interface Props {
    value: string;
    class?: string;
    onValueChange?: (value: string) => void;
    children?: Snippet;
  }

  let { value, class: className = '', onValueChange, children }: Props = $props();

  const valueStore = toStore(() => value);
  const baseId = `tabs-${++tabsRootCount}`;

  setTabsContext({
    baseId,
    value: valueStore,
    setValue: (next) => {
      onValueChange?.(next);
    }
  });
</script>

<div class={cn('w-full', className)} data-slot="tabs-root">
  {@render children?.()}
</div>
