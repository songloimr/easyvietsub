<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import { cn } from '$lib/utils';
  import { onMount } from 'svelte';

  interface Option {
    value: string | number;
    label: string;
  }

  interface Props {
    value?: string | number;
    options: Option[];
    class?: string;
    disabled?: boolean;
    onchange?: (event: Event & { currentTarget: EventTarget & HTMLSelectElement }) => void;
  }

  let { value = '', options, class: className = '', disabled = false, onchange }: Props = $props();

  let open = $state(false);
  let root: HTMLDivElement | null = null;
  let trigger: HTMLButtonElement | null = null;
  const selectedLabel = $derived(
    options.find((option) => String(option.value) === String(value))?.label ?? 'Select option'
  );

  function closeMenu(): void {
    open = false;
  }

  function selectOption(nextValue: string | number): void {
    if (disabled) {
      return;
    }

    value = nextValue;
    open = false;
    queueMicrotask(() => trigger?.focus());

    if (!onchange) {
      return;
    }

    const selectLike = { value: String(nextValue) } as HTMLSelectElement;
    onchange({ currentTarget: selectLike } as Event & {
      currentTarget: EventTarget & HTMLSelectElement;
    });
  }

  function toggle(): void {
    if (disabled) {
      return;
    }

    open = !open;
  }

  function handleOptionPointerDown(event: PointerEvent, nextValue: string | number): void {
    event.preventDefault();
    event.stopPropagation();
    selectOption(nextValue);
  }

  onMount(() => {
    const handlePointerDown = (event: PointerEvent): void => {
      if (root && !root.contains(event.target as Node)) {
        closeMenu();
      }
    };

    window.addEventListener('pointerdown', handlePointerDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
    };
  });
</script>

<div bind:this={root} class={cn('relative', className)}>
  <button
    bind:this={trigger}
    type="button"
    class={cn(
      'flex h-9 w-full items-center justify-between rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50',
      open && 'border-ring ring-[3px] ring-ring/50'
    )}
    {disabled}
    onclick={toggle}
    onkeydown={(event) => {
      if (event.key === 'Escape') {
        closeMenu();
      }
    }}
  >
    <span class="truncate">{selectedLabel}</span>
    <ChevronDown class={cn('size-4 text-muted-foreground transition-transform', open && 'rotate-180')} />
  </button>

  {#if open}
    <div
      class="absolute z-50 mt-1 max-h-64 w-full overflow-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
    >
      {#each options as option}
        <button
          type="button"
          class={cn(
            'flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground',
            String(option.value) === String(value) && 'bg-accent text-accent-foreground'
          )}
          onpointerdown={(event) => handleOptionPointerDown(event, option.value)}
          onkeydown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              selectOption(option.value);
            }

            if (event.key === 'Escape') {
              closeMenu();
            }
          }}
        >
          <span class="truncate">{option.label}</span>
          {#if String(option.value) === String(value)}
            <Check class="size-4 shrink-0" />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
