<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import { cn } from '$lib/utils';

  let { class: className, ...rest }: AlertDialog.TitleProps & { class?: string } = $props();
</script>

<AlertDialog.Title
  class={cn('text-lg font-semibold', className)}
  {...rest}
/>
