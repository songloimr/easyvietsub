<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: number | null;
    size?: number;
    class?: string;
  }

  let { value = null, size = 16, class: className = '' }: Props = $props();

  const stroke = 2;
  const radius = $derived(size / 2 - stroke);
  const circumference = $derived(2 * Math.PI * radius);
  const progress = $derived(value === null ? 25 : Math.min(100, Math.max(0, value)));
  const offset = $derived(circumference - (progress / 100) * circumference);
</script>

<svg
  class={cn(value === null && 'animate-spin', className)}
  width={size}
  height={size}
  viewBox={`0 0 ${size} ${size}`}
  aria-hidden="true"
>
  <circle
    cx={size / 2}
    cy={size / 2}
    r={radius}
    fill="none"
    stroke="currentColor"
    stroke-opacity="0.2"
    stroke-width={stroke}
  />
  <circle
    cx={size / 2}
    cy={size / 2}
    r={radius}
    fill="none"
    stroke="currentColor"
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-dasharray={circumference}
    stroke-dashoffset={offset}
  />
</svg>
