<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    class?: string;
    children?: Snippet;
  }

  let { class: className = '', children }: Props = $props();
</script>

<div
  class={cn(
    'inline-flex h-9 w-fit items-center justify-center rounded-lg bg-muted p-[3px] text-muted-foreground',
    className
  )}
  data-slot="tabs-list"
  aria-orientation="horizontal"
  role="tablist"
>
  {@render children?.()}
</div>
