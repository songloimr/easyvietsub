<script lang="ts">
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import CircleProgress from '$lib/components/ui/circle-progress.svelte';
  import Field from '$lib/components/ui/field.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import {
    deletingModelId,
    downloadModel,
    downloadingModelId,
    downloadingModelProgress,
    installingRuntime,
    installManagedFfmpeg,
    removeManagedFfmpeg,
    removingRuntime,
    runtimeCapabilities,
    runtimeDownloadMessage,
    runtimeDownloadProgress,
    whisperModels
  } from '$lib/stores/app-store';
  import { getCacheSize, cleanupAllCache, cleanupOldCache } from '$lib/services/tauri';
  import { formatBytes } from '$lib/utils';
  import { toast } from 'svelte-sonner';
  import Download from '@lucide/svelte/icons/download';
  import HardDrive from '@lucide/svelte/icons/hard-drive';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  let {
    apiKeyInput = $bindable(''),
    apiConnectionState,
    apiConnectionMessage,
    onSaveApiKey,
    onTestApiConnection,
    onDeleteModel,
    onDeleteRuntime
  }: {
    apiKeyInput: string;
    apiConnectionState: 'idle' | 'testing' | 'success' | 'error';
    apiConnectionMessage: string;
    onSaveApiKey: () => void;
    onTestApiConnection: () => void;
    onDeleteModel: (modelId: string) => void;
    onDeleteRuntime: () => void;
  } = $props();

  let runtimeInstalled = $derived(
    !!$runtimeCapabilities?.ffmpegAvailable || !!$runtimeCapabilities?.localFfmpegInstalled
  );
  let runtimeLabel = $derived(
    $runtimeCapabilities?.ffmpegAvailable
      ? 'FFmpeg sẵn có trong hệ thống'
      : $runtimeCapabilities?.localFfmpegInstalled
        ? 'FFmpeg đã được cài đặt bởi app'
        : 'Chưa cài đặt FFmpeg'
  );
  let runtimeBusy = $derived($installingRuntime || $removingRuntime);

  // Cache management
  let cacheSize = $state<number>(0);
  let loadingCacheSize = $state(false);
  let cleaningCache = $state(false);

  async function loadCacheSize(): Promise<void> {
    loadingCacheSize = true;
    try {
      cacheSize = await getCacheSize();
    } catch (err) {
      console.error('Failed to get cache size:', err);
      toast.error('Không thể lấy kích thước cache');
    } finally {
      loadingCacheSize = false;
    }
  }

  async function handleCleanupAllCache(): Promise<void> {
    cleaningCache = true;
    try {
      await cleanupAllCache();
      cacheSize = 0;
      toast.success('Đã xóa toàn bộ cache');
    } catch (err) {
      console.error('Failed to cleanup cache:', err);
      toast.error('Không thể xóa cache');
    } finally {
      cleaningCache = false;
    }
  }

  async function handleCleanupOldCache(days: number): Promise<void> {
    cleaningCache = true;
    try {
      const cleaned = await cleanupOldCache(days);
      await loadCacheSize();
      toast.success(`Đã xóa ${formatBytes(cleaned)} cache cũ hơn ${days} ngày`);
    } catch (err) {
      console.error('Failed to cleanup old cache:', err);
      toast.error('Không thể xóa cache cũ');
    } finally {
      cleaningCache = false;
    }
  }

  // Load cache size on mount
  loadCacheSize();

</script>

<div class="view-enter space-y-4">
  <!-- API Settings Card -->
  <Card class="border bg-card">
    <div class="p-4">
      <h2 class="mb-4 text-base font-semibold">Cài đặt API</h2>

      <div class="rounded-lg border bg-muted/20 p-4">
        <div class="mb-3 flex items-center gap-2">
          <KeyRound class="size-4" />
          <h3 class="text-sm font-medium">Gemini API key</h3>
        </div>

        <div class="space-y-3">
          <Field label="API key">
            <Input
              value={apiKeyInput}
              oninput={(e) => (apiKeyInput = e.currentTarget.value)}
              placeholder="AIza..."
              class="h-9"
            />
          </Field>

          <div class="flex gap-2">
            <Button onclick={onSaveApiKey} size="sm" class="flex-1">Lưu</Button>
            <Button
              variant="outline"
              disabled={apiConnectionState === 'testing'}
              onclick={onTestApiConnection}
              size="sm"
              class="flex-1"
            >
              {#if apiConnectionState === 'testing'}
                <CircleProgress class="size-4" />
              {/if}
              Test
            </Button>
          </div>

          {#if apiConnectionState !== 'idle'}
            <div
              class={`rounded-md border px-3 py-2 text-sm ${
                apiConnectionState === 'error'
                  ? 'border-destructive/30 bg-destructive/5 text-destructive'
                  : apiConnectionState === 'success'
                    ? 'border-green-500/30 bg-green-500/5 text-green-700 dark:text-green-400'
                    : 'border-border/70 bg-background/70 text-foreground'
              }`}
            >
              {apiConnectionMessage}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </Card>

  <!-- FFmpeg Card -->
  <Card class="border bg-card">
    <div class="p-4">
      <div class="mb-4 flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">FFmpeg</h2>
        <Badge
          variant={runtimeInstalled ? 'default' : 'outline'}
          class={runtimeInstalled ? 'border-green-500/40 bg-green-500/10 text-green-600 dark:text-green-400' : ''}
        >
          {runtimeInstalled ? 'Đã cài' : 'Chưa cài'}
        </Badge>
      </div>

      <div class="space-y-3">
        <div class="rounded-md border bg-muted/20 px-3 py-2 text-sm">
          {runtimeLabel}
        </div>

        <Button
          class="w-full"
          variant={runtimeInstalled ? 'outline' : 'default'}
          disabled={runtimeBusy}
          onclick={runtimeInstalled ? onDeleteRuntime : installManagedFfmpeg}
          size="sm"
        >
          {#if $installingRuntime}
            <CircleProgress class="size-4" />
            Đang cài {$runtimeDownloadProgress}%
          {:else if $removingRuntime}
            <CircleProgress class="size-4" />
            Đang xóa
          {:else if runtimeInstalled}
            <Trash2 class="size-4" />
            Xóa
          {:else}
            <Download class="size-4" />
            Cài đặt
          {/if}
        </Button>

        {#if $installingRuntime || $runtimeDownloadMessage}
          <div class="rounded-md border bg-background/70 px-3 py-2 text-sm">
            {$runtimeDownloadMessage || 'Đang chuẩn bị...'}
          </div>
        {/if}
      </div>
    </div>
  </Card>

  <!-- Whisper Card -->
  <Card class="border bg-card">
    <div class="p-4">
      <div class="mb-4 flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">Whisper</h2>
        <Badge variant="default" class="border-green-500/40 bg-green-500/10 text-green-600 dark:text-green-400">
          Bundled
        </Badge>
      </div>

      <div class="rounded-lg border bg-muted/20 p-4">
        <p class="mb-2 text-sm font-medium">Models</p>
        <p class="mb-3 text-sm text-muted-foreground">
          Tải model Whisper để chuyển giọng nói thành văn bản
        </p>

        <div class="space-y-2">
          {#each $whisperModels as model}
            <div class="flex items-center justify-between gap-3 rounded-md border bg-background/70 px-3 py-2">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium">{model.label}</p>
                <p class="text-xs text-muted-foreground">{formatBytes(model.sizeBytes)}</p>
              </div>

              <div class="flex shrink-0 items-center gap-2">
                {#if $downloadingModelId === model.id}
                  <div class="flex items-center gap-1.5">
                    <CircleProgress class="size-5" />
                    {#if $downloadingModelProgress > 0 && $downloadingModelProgress < 100}
                      <span class="text-xs text-muted-foreground">{$downloadingModelProgress}%</span>
                    {/if}
                  </div>
                {:else if $deletingModelId === model.id}
                  <div class="flex items-center gap-1.5">
                    <CircleProgress class="size-4" />
                    <span class="text-xs text-muted-foreground">Đang xóa...</span>
                  </div>
                {:else if model.downloaded}
                  <Button
                    variant="ghost"
                    size="sm"
                    onclick={() => onDeleteModel(model.id)}
                    class="text-xs"
                  >
                    <Trash2 class="size-4" />
                  </Button>
                {:else}
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={() => downloadModel(model.id)}
                    class="text-xs"
                  >
                    <Download class="size-4" />
                    Tải
                  </Button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </Card>

  <!-- Cache Management Card -->
  <Card class="border bg-card">
    <div class="p-4">
      <div class="mb-4 flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">Cache Management</h2>
        <Badge variant="outline">
          <HardDrive class="mr-1 size-3" />
          {loadingCacheSize ? '...' : formatBytes(cacheSize)}
        </Badge>
      </div>

      <div class="rounded-lg border bg-muted/20 p-4">
        <p class="mb-3 text-sm text-muted-foreground">
          Cache bao gồm file audio tạm và dữ liệu phân đoạn. Xóa cache giúp tiết kiệm dung lượng ổ đĩa.
        </p>

        <div class="space-y-2">
          <Button
            class="w-full"
            variant="outline"
            disabled={cleaningCache}
            onclick={() => handleCleanupOldCache(7)}
            size="sm"
          >
            {#if cleaningCache}
              <CircleProgress class="size-4" />
            {:else}
              <Trash2 class="size-4" />
            {/if}
            Xóa cache cũ hơn 7 ngày
          </Button>

          <Button
            class="w-full"
            variant="outline"
            disabled={cleaningCache}
            onclick={() => handleCleanupOldCache(30)}
            size="sm"
          >
            {#if cleaningCache}
              <CircleProgress class="size-4" />
            {:else}
              <Trash2 class="size-4" />
            {/if}
            Xóa cache cũ hơn 30 ngày
          </Button>

          <Button
            class="w-full"
            variant="destructive"
            disabled={cleaningCache}
            onclick={handleCleanupAllCache}
            size="sm"
          >
            {#if cleaningCache}
              <CircleProgress class="size-4" />
            {:else}
              <Trash2 class="size-4" />
            {/if}
            Xóa toàn bộ cache
          </Button>
        </div>
      </div>
    </div>
  </Card>
</div>
