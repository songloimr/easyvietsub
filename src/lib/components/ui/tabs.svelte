<script lang="ts">
  import type { Component } from 'svelte';
  import { cn } from '$lib/utils';

  interface TabItem {
    value: string;
    label: string;
    icon?: Component<{ class?: string }>;
  }

  interface Props {
    value: string;
    items: TabItem[];
    class?: string;
    triggerClass?: string;
    onchange?: (value: string) => void;
  }

  let {
    value,
    items,
    class: className = '',
    triggerClass = '',
    onchange
  }: Props = $props();
</script>

<div
  class={cn(
    'inline-flex h-10 w-full items-center justify-start gap-1 rounded-md border bg-muted p-1 text-muted-foreground',
    className
  )}
>
  {#each items as item}
    <button
      class={cn(
        'inline-flex min-w-0 flex-1 items-center justify-center gap-2 whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:ring-[3px] focus-visible:ring-ring/50',
        value === item.value && 'bg-background text-foreground shadow-sm',
        triggerClass
      )}
      onclick={() => onchange?.(item.value)}
      type="button"
    >
      {#if item.icon}
        {@const Icon = item.icon}
        <Icon class="size-4 shrink-0" />
      {/if}
      <span class="truncate">{item.label}</span>
    </button>
  {/each}
</div>
