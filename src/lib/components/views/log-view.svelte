<script lang="ts">
  import Card from '$lib/components/ui/card.svelte';
  import { processLogs } from '$lib/stores/app-store';
</script>

<div class="view-enter">
  <Card class="border bg-card">
    <div class="p-4">
      <div class="mb-4 flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">Error Logs ({$processLogs.length})</h2>
      </div>

      {#if $processLogs.length === 0}
        <div class="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
          Chưa có lỗi nào.
        </div>
      {:else}
        <div class="space-y-2">
          {#each $processLogs as item}
            <div class="rounded-lg border border-destructive/20 bg-destructive/5 p-3">
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-medium text-foreground">{item.context}</p>
                  <p class="mt-0.5 text-xs text-destructive/80">{item.message}</p>
                </div>
                <p class="shrink-0 text-xs text-muted-foreground">
                  {new Date(item.createdAt).toLocaleString('vi-VN', { timeStyle: 'short' })}
                </p>
              </div>

              {#if item.detail}
                <pre class="mt-2 overflow-x-auto whitespace-pre-wrap wrap-break-word rounded-md border border-destructive/10 bg-background/70 px-2.5 py-1.5 text-xs text-foreground">{item.detail}</pre>
              {/if}

              {#if item.stack}
                <details class="mt-1.5">
                  <summary class="cursor-pointer text-xs text-muted-foreground hover:text-foreground">Stack trace</summary>
                  <pre class="mt-1 overflow-x-auto whitespace-pre-wrap wrap-break-word rounded-md border bg-background/70 px-2.5 py-1.5 text-xs opacity-60">{item.stack}</pre>
                </details>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </Card>
</div>
