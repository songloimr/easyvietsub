<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Search from '@lucide/svelte/icons/search';
  import { cn } from '$lib/utils';
  import { onMount } from 'svelte';

  interface Option {
    value: string;
    label: string;
    style?: string;
  }

  interface Props {
    value?: string;
    options: Option[];
    class?: string;
    disabled?: boolean;
    placeholder?: string;
    searchPlaceholder?: string;
    emptyLabel?: string;
    onchange?: (value: string) => void;
  }

  let {
    value = '',
    options,
    class: className = '',
    disabled = false,
    placeholder = 'Select option',
    searchPlaceholder = 'Search...',
    emptyLabel = 'No results found.',
    onchange
  }: Props = $props();

  let open = $state(false);
  let query = $state('');
  let root: HTMLDivElement | null = null;
  let trigger: HTMLButtonElement | null = null;
  let input = $state<HTMLInputElement | null>(null);

  const filteredOptions = $derived.by(() => {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) {
      return options;
    }

    return options.filter((option) => option.label.toLowerCase().includes(normalizedQuery));
  });

  const selectedLabel = $derived(
    options.find((option) => option.value === value)?.label ?? placeholder
  );

  function closeMenu(): void {
    open = false;
    query = '';
  }

  function handleToggle(): void {
    if (disabled) {
      return;
    }

    open = !open;
    if (open) {
      queueMicrotask(() => input?.focus());
    } else {
      query = '';
    }
  }

  function handleSelect(nextValue: string): void {
    value = nextValue;
    closeMenu();
    queueMicrotask(() => trigger?.focus());
    onchange?.(nextValue);
  }

  function handleOptionPointerDown(event: PointerEvent, nextValue: string): void {
    event.preventDefault();
    event.stopPropagation();
    handleSelect(nextValue);
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
    onclick={handleToggle}
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
    <div class="absolute z-50 mt-1 w-full rounded-md border bg-popover p-1 text-popover-foreground shadow-md">
      <div class="flex items-center gap-2 border-b px-2 py-2">
        <Search class="size-4 text-muted-foreground" />
        <input
          bind:this={input}
          bind:value={query}
          type="text"
          class="h-8 w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          placeholder={searchPlaceholder}
          onkeydown={(event) => {
            if (event.key === 'Escape') {
              closeMenu();
            }
          }}
        />
      </div>

      <div class="max-h-64 overflow-auto p-1">
        {#if filteredOptions.length === 0}
          <div class="px-2 py-3 text-sm text-muted-foreground">{emptyLabel}</div>
        {:else}
          {#each filteredOptions as option}
            <button
              type="button"
              class={cn(
                'flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground',
                option.value === value && 'bg-accent text-accent-foreground'
              )}
              onpointerdown={(event) => handleOptionPointerDown(event, option.value)}
              onkeydown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  handleSelect(option.value);
                }

                if (event.key === 'Escape') {
                  closeMenu();
                }
              }}
            >
              <span class="truncate" style={option.style ?? ''}>{option.label}</span>
              {#if option.value === value}
                <Check class="size-4 shrink-0" />
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>
