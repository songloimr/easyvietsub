<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { cn } from '$lib/utils';

  let { class: className, ...rest }: Dialog.DescriptionProps & { class?: string } = $props();
</script>

<Dialog.Description
  class={cn('text-sm text-muted-foreground', className)}
  {...rest}
/>
