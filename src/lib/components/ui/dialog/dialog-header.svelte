<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  let {
    class: className,
    children,
    ...rest
  }: HTMLAttributes<HTMLDivElement> & { class?: string; children?: Snippet } = $props();
</script>

<div class={cn('flex flex-col space-y-1.5 text-center sm:text-left', className)} {...rest}>
  {#if children}
    {@render children()}
  {/if}
</div>
