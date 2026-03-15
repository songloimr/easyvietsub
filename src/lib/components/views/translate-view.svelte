<script lang="ts">
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import CircleProgress from '$lib/components/ui/circle-progress.svelte';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import Field from '$lib/components/ui/field.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Textarea from '$lib/components/ui/textarea.svelte';
  import {
    DEFAULT_FORM,
    FALLBACK_GEMINI_MODELS,
    SOURCE_LANGUAGE_OPTIONS
  } from '$lib/constants';
  import {
    activeJob,
    busy,
    cancelModelDownload,
    deleteModel,
    deletingModelId,
    downloadModel,
    downloadingModelId,
    downloadingModelProgress,
    inspection,
    inspectSelectedMedia,
    retranslateOnly,
    runtimeCapabilities,
    startPipeline,
    statusMessage,
    updateForm,
    updateSubtitleSegment,
    validationErrors,
    whisperModels
  } from '$lib/stores/app-store';
  import { openPathSelection, savePathSelection, writeTextContent, exportAssSubtitle as exportAssSubtitleService } from '$lib/services/tauri';
  import { segmentsToSrt } from '$lib/srt';
  import { formatBytes, formatDuration, formatTimestamp } from '$lib/utils';
  import { toast } from 'svelte-sonner';
  import AudioLines from '@lucide/svelte/icons/audio-lines';
  import BookAudio from '@lucide/svelte/icons/book-audio';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Download from '@lucide/svelte/icons/download';
  import FileText from '@lucide/svelte/icons/file-text';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Languages from '@lucide/svelte/icons/languages';
  import Play from '@lucide/svelte/icons/play';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Video from '@lucide/svelte/icons/video';
  import AlertTriangle from '@lucide/svelte/icons/triangle-alert';
  import X from '@lucide/svelte/icons/x';
  import Zap from '@lucide/svelte/icons/zap';

  let {
    apiKeyInput = $bindable(''),
    onConfirmRetranslate,
    onConfirmRetranslateOnly,
    onDeleteModel
  }: {
    apiKeyInput: string;
    onConfirmRetranslate: () => void;
    onConfirmRetranslateOnly: () => void;
    onDeleteModel: (modelId: string) => void;
  } = $props();

  let inspectingMedia = $state(false);

  let selectedWhisperModel = $derived(
    $whisperModels.find((model) => model.id === $activeJob.form.whisperModelId) ?? $whisperModels[0]
  );
  let canStart = $derived(
    !$busy &&
    !!$activeJob.form.inputPath &&
    ($activeJob.form.processingMode === 'gemini_direct' || selectedWhisperModel?.downloaded) &&
    !!apiKeyInput.trim()
  );
  let canExportSubtitles = $derived(
    ($activeJob.status === 'completed' || $activeJob.status === 'partial') &&
    $activeJob.sourceSegments.length > 0 &&
    $activeJob.translatedSegments.length > 0
  );
  let isPartialTranslation = $derived($activeJob.status === 'partial');
  let canRetranslate = $derived(
    ($activeJob.status === 'completed' || $activeJob.status === 'partial') &&
    $activeJob.sourceSegments.length > 0 &&
    $activeJob.form.processingMode === 'whisper_translate' &&
    !$busy
  );
  let translatedSegmentCount = $derived($activeJob.translatedSegments.filter(s => s.translatedText.trim() !== '').length);
  let usesWhisper = $derived($activeJob.form.processingMode === 'whisper_translate');
  let ffmpegReady = $derived(
    !!$runtimeCapabilities?.ffmpegAvailable || !!$runtimeCapabilities?.localFfmpegInstalled
  );
  let whisperReady = $derived(true);

  async function handleSelectMedia(): Promise<void> {
    const path = await openPathSelection(false, [
      {
        name: 'Media',
        extensions: ['mp4', 'mov', 'mkv', 'mp3', 'wav', 'm4a', 'aac']
      }
    ]);
    if (!path) return;
    inspectingMedia = true;
    try {
      await inspectSelectedMedia(path);
    } finally {
      inspectingMedia = false;
    }
  }

  async function handleStartPipeline(): Promise<void> {
    if ($activeJob.translatedSegments.length > 0) {
      onConfirmRetranslate();
      return;
    }
    await startPipeline(apiKeyInput);
  }

  async function handleRetranslateOnly(): Promise<void> {
    onConfirmRetranslateOnly();
  }

  async function exportSubtitle(kind: 'source' | 'translated'): Promise<void> {
    const segments = kind === 'source' ? $activeJob.sourceSegments : $activeJob.translatedSegments;
    if (segments.length === 0) return;

    if (kind === 'translated' && isPartialTranslation) {
      toast.warning('Gemini dịch thất bại — file xuất ra sẽ chứa ngôn ngữ gốc, không phải tiếng Việt.');
    }

    try {
      const content = segmentsToSrt(segments, kind === 'translated');
      const baseName = ($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '');
      const filename = `${baseName}.${kind === 'source' ? 'source' : 'vi'}.srt`;
      const path = await savePathSelection(filename);
      if (!path) return;
      await writeTextContent(path, content);
      toast.success(`Đã xuất file ${filename}`);
    } catch (err) {
      console.error('Lỗi khi xuất SRT:', err);
      const msg = err instanceof Error ? err.message : 'Lỗi không xác định';
      toast.error(`Xuất SRT thất bại: ${msg}`);
    }
  }

  async function exportAssSubtitle(): Promise<void> {
    const segments = $activeJob.translatedSegments;
    if (segments.length === 0) return;

    if (isPartialTranslation) {
      toast.warning('Gemini dịch thất bại — file ASS xuất ra sẽ chứa ngôn ngữ gốc, không phải tiếng Việt.');
    }

    try {
      const baseName = ($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '');
      const filename = `${baseName}.ass`;
      const path = await savePathSelection(filename);
      if (!path) return;
      await exportAssSubtitleService({
        path,
        segments,
        style: $activeJob.style
      });
      toast.success(`Đã xuất file ${filename}`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Lỗi không xác định';
      toast.error(`Xuất ASS thất bại: ${msg}`);
    }
  }

</script>

<div class="view-enter space-y-3">
  <!-- Compact Config Area -->
  <div class="rounded-lg border bg-card p-3">
    <div class="space-y-3">
      <!-- Row 1: File input + Browse + Start -->
      <div class="flex items-start gap-2">
        <div class="flex-1 min-w-0">
          <Field label="Input file">
            <div class="flex items-center gap-2">
              <Input value={$activeJob.form.inputPath} disabled class="flex-1" />
              <Button
                variant="outline"
                size="icon"
                title="Browse input"
                ariaLabel="Browse input"
                disabled={inspectingMedia}
                onclick={handleSelectMedia}
              >
                {#if inspectingMedia}
                  <CircleProgress class="size-4" />
                {:else}
                  <FolderOpen class="size-4" />
                {/if}
              </Button>
            </div>
            {#if inspectingMedia}
              <p class="mt-1 text-xs text-muted-foreground animate-pulse">
                Đang đọc metadata media...
              </p>
            {/if}
          </Field>
        </div>
        <div class="pt-6">
          <Button disabled={!canStart || !ffmpegReady || (usesWhisper && !whisperReady)} onclick={handleStartPipeline}>
            <Play class="size-4" />
            Bắt đầu
          </Button>
        </div>
      </div>

      <!-- Media info badges (inline, compact) -->
      {#if $inspection}
        <div class="flex items-center gap-2 flex-wrap">
          <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1 text-xs">
            {#if $inspection.kind === 'video'}
              <Video class="size-3" />
            {:else}
              <AudioLines class="size-3" />
            {/if}
            <span class="font-medium">{$inspection.kind}</span>
          </div>
          <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1 text-xs">
            <BookAudio class="size-3" />
            <span class="font-medium">{formatDuration($inspection.durationSeconds)}</span>
          </div>
          <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1 text-xs">
            <Download class="size-3" />
            <span class="font-medium">{$inspection.audioTracks.length} tracks</span>
          </div>
          {#if $activeJob.form.processingMode === 'gemini_direct'}
            <Badge variant="outline" class="border-amber-500/50 text-xs text-amber-700">
              Experimental
            </Badge>
          {/if}
        </div>
      {/if}

      <!-- Row 2: Config selects (2-column layout) -->
      <div class="{usesWhisper ? 'grid grid-cols-2 gap-4' : 'grid grid-cols-1'}">
        <!-- Column 1: General settings -->
        <div class="space-y-2">
          <Field label="Source language">
            <Select
              value={$activeJob.form.sourceLanguage}
              options={SOURCE_LANGUAGE_OPTIONS}
              onchange={(event) => updateForm('sourceLanguage', event.currentTarget.value)}
            />
          </Field>
          <Field label="Processing mode">
            <Select
              value={$activeJob.form.processingMode}
              options={[
                { value: 'whisper_translate', label: 'Whisper + Gemini' },
                { value: 'gemini_direct', label: 'Gemini direct' }
              ]}
              onchange={(event) =>
                updateForm(
                  'processingMode',
                  event.currentTarget.value as typeof DEFAULT_FORM.processingMode
                )
              }
            />
          </Field>
          <Field label="Gemini model">
            <Select
              value={$activeJob.form.geminiModelId}
              options={FALLBACK_GEMINI_MODELS.map((model) => ({
                value: model.id,
                label: model.label
              }))}
              onchange={(event) => updateForm('geminiModelId', event.currentTarget.value)}
            />
          </Field>
          {#if $inspection && $inspection.audioTracks.length > 1}
            <Field label="Audio track">
              <Select
                value={$activeJob.form.selectedAudioTrack}
                options={$inspection.audioTracks.map((track) => ({
                  value: track.index,
                  label: `Track ${track.index} • ${track.language || 'unknown'}`
                }))}
                onchange={(event) => updateForm('selectedAudioTrack', Number(event.currentTarget.value))}
              />
            </Field>
          {/if}
        </div>

        <!-- Column 2: Whisper settings (conditional) -->
        {#if usesWhisper}
          <div class="space-y-2">
            <Field label="CPU only">
              <button
                type="button"
                role="switch"
                aria-label="CPU only"
                aria-checked={$activeJob.form.cpuOnly}
                class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background {$activeJob.form.cpuOnly ? 'bg-primary' : 'bg-input'}"
                onclick={() => updateForm('cpuOnly', !$activeJob.form.cpuOnly)}
              >
                <span
                  class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-background shadow-lg ring-0 transition duration-200 ease-in-out {$activeJob.form.cpuOnly ? 'translate-x-5' : 'translate-x-0'}"
                ></span>
              </button>
            </Field>

            <Field label="Whisper model">
              <div class="space-y-2">
                <Select
                  value={$activeJob.form.whisperModelId}
                  disabled={$busy || $downloadingModelId !== null}
                  options={$whisperModels.map((model) => ({
                    value: model.id,
                    label: `${model.label} • ${formatBytes(model.sizeBytes)}`
                  }))}
                  onchange={(event) => updateForm('whisperModelId', event.currentTarget.value)}
                />

                {#if selectedWhisperModel}
                  <div class="flex items-center justify-between gap-2 rounded-md border bg-background/70 px-2.5 py-1.5">
                    <p class="truncate text-sm font-medium">{selectedWhisperModel.label}</p>
                    <div class="flex items-center gap-1.5">
                      <Button
                        variant={selectedWhisperModel.downloaded ? 'secondary' : 'outline'}
                        size="sm"
                        disabled={$busy || selectedWhisperModel.downloaded || ($downloadingModelId !== null && $downloadingModelId !== selectedWhisperModel.id)}
                        onclick={() => downloadModel(selectedWhisperModel.id)}
                      >
                        {#if $downloadingModelId === selectedWhisperModel.id}
                          <CircleProgress value={$downloadingModelProgress} class="size-3.5" />
                          {$downloadingModelProgress}%
                        {:else}
                          {selectedWhisperModel.downloaded ? 'Đã tải' : 'Tải model'}
                        {/if}
                      </Button>
                      {#if $downloadingModelId === selectedWhisperModel.id}
                        <Button
                          variant="outline"
                          size="sm"
                          title="Hủy tải model"
                          ariaLabel="Hủy tải model"
                          onclick={cancelModelDownload}
                        >
                          <X class="size-3.5" />
                        </Button>
                      {:else if selectedWhisperModel.downloaded}
                        <Button
                          variant="outline"
                          size="sm"
                          title="Delete downloaded model"
                          ariaLabel="Delete downloaded model"
                          disabled={$busy}
                          onclick={() => onDeleteModel(selectedWhisperModel.id)}
                        >
                          <Trash2 class="size-3.5" />
                        </Button>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            </Field>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Status Strip -->
  <div class="rounded-lg border bg-muted/30 px-3 py-2">
    <div class="flex items-center justify-between gap-3 flex-wrap">
      <div class="flex items-center gap-3 flex-wrap flex-1 min-w-0">
        <p class="truncate text-sm text-muted-foreground">{$statusMessage}</p>
        {#if !ffmpegReady}
          <div class="flex items-center gap-1.5 text-xs text-yellow-700 dark:text-yellow-400">
            <AlertTriangle class="size-3.5 shrink-0" />
            <span>FFmpeg required</span>
          </div>
        {/if}
        {#if usesWhisper && !whisperReady}
          <div class="flex items-center gap-1.5 text-xs text-yellow-700 dark:text-yellow-400">
            <AlertTriangle class="size-3.5 shrink-0" />
            <span>Whisper model required</span>
          </div>
        {/if}
      </div>
      {#if $activeJob.tokenUsage}
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground whitespace-nowrap">
          <Zap class="size-3" />
          <span>{$activeJob.tokenUsage.totalTokens.toLocaleString()} tokens</span>
          <span class="text-muted-foreground/50">
            (in: {$activeJob.tokenUsage.promptTokens.toLocaleString()} 
            / out: {$activeJob.tokenUsage.completionTokens.toLocaleString()})
          </span>
        </div>
      {/if}
    </div>
  </div>

  <!-- Subtitle Editor -->
  <Collapsible.Root open class="group/editor rounded-lg border bg-card">
    <div class="flex items-center justify-between gap-2 p-4">
      <h2 class="text-base font-semibold">Subtitle Editor</h2>
      <div class="flex items-center gap-2">
        {#if canRetranslate}
          <Button variant="outline" size="sm" onclick={handleRetranslateOnly} class="text-xs">
            <RefreshCw class="size-3.5" />
            <span>Dịch lại</span>
          </Button>
        {/if}
        {#if canExportSubtitles}
          <div class="flex gap-1.5">
            <Button variant="outline" size="sm" onclick={() => exportSubtitle('source')} class="text-xs">
              <Languages class="size-3.5" />
              <span>Source</span>
            </Button>
            <Button variant="outline" size="sm" onclick={() => exportSubtitle('translated')} class="text-xs">
              <Languages class="size-3.5" />
              <span>Vietnamese</span>
            </Button>
            <Button variant="outline" size="sm" onclick={exportAssSubtitle} class="text-xs">
              <FileText class="size-3.5" />
              <span>ASS</span>
            </Button>
          </div>
        {/if}
        <Collapsible.Trigger class="rounded-md p-1 hover:bg-muted">
          <ChevronDown class="size-4 transition-transform group-data-[state=open]/editor:rotate-180" />
        </Collapsible.Trigger>
      </div>
    </div>
    <Collapsible.Content class="px-4 pb-4">

    {#if $validationErrors.length > 0}
      <div class="mb-3 rounded-lg border border-destructive/30 bg-destructive/5 p-2.5 text-xs text-destructive">
        {#each $validationErrors as error}
          <p>{error}</p>
        {/each}
      </div>
    {/if}

    {#if isPartialTranslation && translatedSegmentCount > 0}
      <div class="mb-3 flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2.5 text-sm text-yellow-700 dark:text-yellow-400">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>Gemini dịch thất bại. Subtitle hiện tại là ngôn ngữ gốc. Bấm <strong>Dịch lại</strong> để thử lại với Gemini.</span>
      </div>
    {/if}

    <div class="soft-scrollbar max-h-[calc(100vh-18rem)] space-y-1.5 overflow-auto pr-1">
      {#if translatedSegmentCount === 0}
      <div class="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
        Chạy pipeline để tạo subtitle.
      </div>
      {:else}
        {#each $activeJob.translatedSegments.filter(s => s.translatedText.trim() !== '') as segment, filterIndex}
          {@const originalIndex = $activeJob.translatedSegments.findIndex(s => s.id === segment.id)}
          <div class="rounded-lg border bg-background/70">
            <div class="flex items-center gap-2 border-b px-2.5 py-1.5">
              <span class="w-6 shrink-0 text-center text-md font-semibold text-muted-foreground">
                {originalIndex + 1}
              </span>
              <div class="flex flex-1 items-center gap-1.5">
                <Input
                  value={formatTimestamp(segment.startMs)}
                  placeholder="00:00:00,000"
                  disabled
                  class="h-6 w-30 shrink-0 text-center font-mono text-xs"
                />
                <span class="text-xs text-muted-foreground">-></span>
                <Input
                  value={formatTimestamp(segment.endMs)}
                  placeholder="00:00:00,000"
                  disabled
                  class="h-6 w-30 shrink-0 text-center font-mono text-xs"
                />
              </div>
              <span class="shrink-0 text-sm tabular-nums text-muted-foreground">
                {formatDuration((segment.endMs - segment.startMs) / 1000)}
              </span>
            </div>

            <div class="p-1.5">
              <Textarea
                value={segment.translatedText}
                rows={2}
                placeholder="Vietnamese..."
                oninput={(event) =>
                  updateSubtitleSegment(segment.id, { translatedText: event.currentTarget.value })}
                class="resize-none border-0 bg-transparent p-0 text-sm shadow-none focus-visible:ring-0"
              />
            </div>
          </div>
        {/each}
      {/if}
    </div>
    </Collapsible.Content>
  </Collapsible.Root>
</div>
