<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { getTabsContext } from './context';

  interface Props {
    value: string;
    class?: string;
    children?: Snippet;
  }

  let { value, class: className = '', children }: Props = $props();

  const { baseId, value: valueStore } = getTabsContext();
</script>

{#if $valueStore === value}
  <div
    class={cn('mt-3 outline-none', className)}
    id={`${baseId}-content-${value}`}
    data-slot="tabs-content"
    data-state="active"
    role="tabpanel"
    tabindex="0"
    aria-labelledby={`${baseId}-trigger-${value}`}
  >
    {@render children?.()}
  </div>
{/if}
