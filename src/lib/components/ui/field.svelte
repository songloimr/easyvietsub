<script lang="ts">
  interface Props {
    label: string;
    description?: string;
    children?: import('svelte').Snippet;
  }

  let { label, description = '', children }: Props = $props();
</script>

<label class="grid gap-2">
  <div class="grid gap-1.5">
    <p class="text-sm leading-none font-medium">{label}</p>
    {#if description}
      <p class="text-xs text-muted-foreground">{description}</p>
    {/if}
  </div>
  {@render children?.()}
</label>
