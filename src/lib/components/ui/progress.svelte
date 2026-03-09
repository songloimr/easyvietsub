<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: number;
    class?: string;
  }

  let { value = 0, class: className = '' }: Props = $props();
</script>

<div class={cn('relative h-2 w-full overflow-hidden rounded-full bg-primary/20', className)}>
  <div
    class="h-full w-full flex-1 bg-primary transition-all"
    style={`width: ${Math.min(100, Math.max(0, value))}%`}
  ></div>
</div>
