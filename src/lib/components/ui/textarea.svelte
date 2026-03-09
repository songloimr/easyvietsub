<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: string;
    class?: string;
    placeholder?: string;
    rows?: number;
    disabled?: boolean;
    oninput?: (event: Event & { currentTarget: EventTarget & HTMLTextAreaElement }) => void;
  }

  let {
    value = '',
    class: className = '',
    placeholder = '',
    rows = 4,
    disabled = false,
    oninput
  }: Props = $props();
</script>

<textarea
  bind:value
  {placeholder}
  {rows}
  {disabled}
  {oninput}
  class={cn(
    'field-sizing-content flex min-h-16 w-full rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
    className
  )}
></textarea>
