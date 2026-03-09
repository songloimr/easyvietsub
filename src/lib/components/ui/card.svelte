<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    class?: string;
    children?: import('svelte').Snippet;
  }

  let { class: className = '', children }: Props = $props();
</script>

<section class={cn('rounded-lg border bg-card text-card-foreground shadow-sm', className)}>
  {@render children?.()}
</section>
