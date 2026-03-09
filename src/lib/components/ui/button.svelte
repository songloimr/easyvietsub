<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    type?: 'button' | 'submit' | 'reset';
    variant?: 'default' | 'secondary' | 'outline' | 'ghost' | 'destructive' | 'link' | 'success';
    size?: 'default' | 'sm' | 'lg' | 'icon';
    class?: string;
    disabled?: boolean;
    title?: string;
    ariaLabel?: string;
    onclick?: (event: MouseEvent) => void;
    children?: import('svelte').Snippet;
  }

  let {
    type = 'button',
    variant = 'default',
    size = 'default',
    class: className = '',
    disabled = false,
    title = '',
    ariaLabel = '',
    onclick,
    children
  }: Props = $props();

  const variants = {
    default: 'bg-primary text-primary-foreground shadow-xs hover:bg-primary/90',
    secondary: 'bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80',
    outline: 'border border-input bg-background shadow-xs hover:bg-accent hover:text-accent-foreground',
    ghost: 'hover:bg-accent hover:text-accent-foreground',
    destructive: 'bg-destructive text-white shadow-xs hover:bg-destructive/90',
    link: 'text-primary underline-offset-4 hover:underline',
    success: 'bg-green-600 text-white shadow-xs hover:bg-green-700'
  };

  const sizes = {
    default: 'h-9 px-4 py-2',
    sm: 'h-8 rounded-md px-3 text-xs',
    lg: 'h-10 rounded-md px-8',
    icon: 'size-9'
  };
</script>

<button
  {type}
  {disabled}
  {title}
  {onclick}
  aria-label={ariaLabel || title || undefined}
  class={cn(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0',
    variants[variant],
    sizes[size],
    className
  )}
>
  {@render children?.()}
</button>
