<script lang="ts">
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import CircleProgress from '$lib/components/ui/circle-progress.svelte';
  import { cancelRequested, statusMessage } from '$lib/stores/app-store';
  import Square from '@lucide/svelte/icons/square';

  let {
    pipelineElapsedSeconds,
    onCancel
  }: {
    pipelineElapsedSeconds: number;
    onCancel: () => void;
  } = $props();

  function formatElapsed(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-background/60 px-4 backdrop-blur-md">
  <Card class="w-full max-w-sm border bg-card/90 p-6 shadow-xl">
    <div class="flex flex-col items-center text-center">
      <CircleProgress class="text-primary" size={88} />
      <p class="mt-4 text-sm font-medium">{$statusMessage}</p>
      <p class="mt-1 font-mono text-xs text-muted-foreground">{formatElapsed(pipelineElapsedSeconds)}</p>

      <Button class="mt-5 w-full" variant="outline" disabled={$cancelRequested} onclick={onCancel}>
        <Square class="size-4" />
        {$cancelRequested ? 'Đang hủy...' : 'Hủy'}
      </Button>
    </div>
  </Card>
</div>
