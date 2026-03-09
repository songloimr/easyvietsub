<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import { cn } from '$lib/utils';
  import { onMount } from 'svelte';

  interface MenubarItem {
    label?: string;
    shortcut?: string;
    disabled?: boolean;
    destructive?: boolean;
    separator?: boolean;
    onselect?: () => void | Promise<void>;
  }

  interface MenubarMenu {
    label: string;
    items: MenubarItem[];
  }

  interface Props {
    menus: MenubarMenu[];
    class?: string;
  }

  let { menus, class: className = '' }: Props = $props();
  let root: HTMLDivElement | null = null;
  let openMenu = $state(-1);

  function closeAll(): void {
    openMenu = -1;
  }

  function toggleMenu(index: number): void {
    openMenu = openMenu === index ? -1 : index;
  }

  async function handleSelect(item: MenubarItem): Promise<void> {
    if (item.disabled || item.separator || !item.onselect) {
      return;
    }

    closeAll();
    await item.onselect();
  }

  onMount(() => {
    const handlePointerDown = (event: PointerEvent): void => {
      if (root && !root.contains(event.target as Node)) {
        closeAll();
      }
    };

    window.addEventListener('pointerdown', handlePointerDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
    };
  });
</script>

<div
  bind:this={root}
  class={cn('inline-flex h-9 items-center rounded-md border bg-background p-1', className)}
>
  {#each menus as menu, index}
    <div class="relative">
      <button
        type="button"
        class={cn(
          'inline-flex items-center gap-1 rounded-sm px-3 py-1.5 text-sm font-medium outline-none transition-colors hover:bg-accent hover:text-accent-foreground',
          openMenu === index && 'bg-accent text-accent-foreground'
        )}
        onclick={() => toggleMenu(index)}
      >
        <span>{menu.label}</span>
        <ChevronDown class="size-3.5" />
      </button>

      {#if openMenu === index}
        <div class="absolute left-0 z-50 mt-1 min-w-52 rounded-md border bg-popover p-1 text-popover-foreground shadow-md">
          {#each menu.items as item}
            {#if item.separator}
              <div class="my-1 h-px bg-border"></div>
            {:else}
              <button
                type="button"
                class={cn(
                  'flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none disabled:opacity-50',
                  item.destructive && 'text-destructive hover:text-destructive'
                )}
                disabled={item.disabled}
                onclick={() => handleSelect(item)}
              >
                <span>{item.label}</span>
                {#if item.shortcut}
                  <span class="text-xs text-muted-foreground">{item.shortcut}</span>
                {/if}
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>
