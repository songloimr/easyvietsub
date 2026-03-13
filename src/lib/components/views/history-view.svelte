<script lang="ts">
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import {
    deleteHistoryItem,
    history,
    loadJobFromHistory,
    setView
  } from '$lib/stores/app-store';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  let {
    onClearHistory,
    onLoadJob
  }: {
    onClearHistory: () => void;
    onLoadJob: (jobId: string) => void;
  } = $props();
</script>

<div class="view-enter">
  <Card class="border bg-card">
    <div class="p-4">
      <div class="mb-4 flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">History ({$history.length})</h2>
        {#if $history.length > 0}
          <Button variant="outline" size="sm" onclick={onClearHistory} class="text-xs">
            <Trash2 class="size-3.5" />
            <span>Xóa tất cả</span>
          </Button>
        {/if}
      </div>

      {#if $history.length === 0}
        <div class="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
          Chưa có job nào.
        </div>
      {:else}
        <div class="space-y-1.5">
          {#each $history as item}
            <div class="flex w-full items-center gap-2 rounded-lg border bg-background/70 px-3 py-2.5 text-sm transition-colors duration-150 hover:bg-accent/40">
              <button
                class="flex flex-1 items-center justify-between gap-2 text-left"
                onclick={() => {
                  setView('translate');
                  loadJobFromHistory(item);
                  onLoadJob(item.id);
                }}
                type="button"
              >
                <div class="min-w-0 flex-1">
                  <p class="truncate font-medium">{item.name}</p>
                  <p class="mt-0.5 text-xs text-muted-foreground">
                    {new Date(item.updatedAt).toLocaleString('vi-VN', { dateStyle: 'short', timeStyle: 'short' })}
                  </p>
                </div>
                <div class="flex items-center gap-2.5">
                  <span class="text-xs text-muted-foreground">{item.translatedSegments.length} segs</span>
                  {#if item.tokenUsage}
                    <span class="text-xs text-muted-foreground">{item.tokenUsage.totalTokens.toLocaleString()} tokens</span>
                  {/if}
                  <Badge 
                    variant={item.status === 'completed' ? 'default' : item.status === 'partial' ? 'secondary' : 'outline'} 
                    class="text-xs"
                  >
                    {item.phase}
                  </Badge>
                </div>
              </button>
              <Button
                variant="ghost"
                size="icon"
                class="shrink-0"
                title="Xóa"
                ariaLabel="Xóa job này"
                onclick={(event) => {
                  event.stopPropagation();
                  deleteHistoryItem(item.id);
                }}
              >
                <Trash2 class="size-4 text-muted-foreground hover:text-destructive" />
              </Button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </Card>
</div>
