<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import { cn } from '$lib/utils';

  let { class: className, ...rest }: AlertDialog.DescriptionProps & { class?: string } = $props();
</script>

<AlertDialog.Description
  class={cn('text-sm text-muted-foreground', className)}
  {...rest}
/>
