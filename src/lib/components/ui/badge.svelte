<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    variant?: 'default' | 'secondary' | 'outline' | 'destructive';
    class?: string;
    children?: import('svelte').Snippet;
  }

  let { variant = 'default', class: className = '', children }: Props = $props();

  const variants = {
    default: 'border-transparent bg-primary text-primary-foreground',
    secondary: 'border-transparent bg-secondary text-secondary-foreground',
    outline: 'text-foreground',
    destructive: 'border-transparent bg-destructive text-white'
  };
</script>

<span
  class={cn(
    'inline-flex w-fit items-center justify-center gap-1 whitespace-nowrap rounded-md border px-2 py-0.5 text-xs font-medium transition-colors',
    variants[variant],
    className
  )}
>
  {@render children?.()}
</span>
