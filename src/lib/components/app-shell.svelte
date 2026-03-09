<script lang="ts">
  import PreviewCanvas from '$lib/components/preview-canvas.svelte';
  import TiptapEditor from '$lib/components/tiptap-editor.svelte';
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import CircleProgress from '$lib/components/ui/circle-progress.svelte';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import Field from '$lib/components/ui/field.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Menubar from '$lib/components/ui/menubar.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import Textarea from '$lib/components/ui/textarea.svelte';
  import {
    DEFAULT_FORM,
    FALLBACK_GEMINI_MODELS,
    SOURCE_LANGUAGE_OPTIONS,
    VIEW_LABELS
  } from '$lib/constants';
  import {
    exportAssSubtitle as exportAssSubtitleService,
    fetchSupportedGeminiModels,
    openPathSelection,
    savePathSelection,
    writeTextContent
  } from '$lib/services/tauri';
  import {
    activeJob,
    bootstrapApp,
    busy,
    cancelRequested,
    clearHistory,
    currentView,
    deleteHistoryItem,
    deleteModel,
    downloadModel,
    downloadingModelId,
    deletingModelId,
    downloadingModelProgress,
    exportProject,
    history,
    importProject,
    installingRuntime,
    installManagedFfmpeg,
    inspectSelectedMedia,
    inspection,
    loadJobFromHistory,
    removeManagedFfmpeg,
    removeManagedWhisper,
    recordProcessError,
    removingRuntime,
    renderCurrentVideo,
    processLogs,
    requestCancellation,
    restoreSavedApiKey,
    runtimeCapabilities,
    runtimeDownloadMessage,
    runtimeDownloadProgress,
    saveApiKey,
    saveTranslationInstruction,
    settings,
    setView,
    startPipeline,
    statusMessage,
    updateForm,
    updateStyle,
    updateSubtitleSegment,
    validationErrors,
    whisperModels
  } from '$lib/stores/app-store';
  import { segmentsToSrt } from '$lib/srt';
  import type { SubtitleSegment, ViewKey } from '$lib/types';
  import {
    downloadTextFile,
    formatBytes,
    formatDuration,
    formatTimestamp,
    parseTimestamp
  } from '$lib/utils';
  import AudioLines from '@lucide/svelte/icons/audio-lines';
  import BookAudio from '@lucide/svelte/icons/book-audio';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Download from '@lucide/svelte/icons/download';
  import FileText from '@lucide/svelte/icons/file-text';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import Languages from '@lucide/svelte/icons/languages';
  import Play from '@lucide/svelte/icons/play';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import Square from '@lucide/svelte/icons/square';
  import Subtitles from '@lucide/svelte/icons/subtitles';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import AlertTriangle from '@lucide/svelte/icons/triangle-alert';
  import Video from '@lucide/svelte/icons/video';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';
  import { onMount } from 'svelte';

  type NavItem = {
    key: ViewKey;
    icon: typeof FolderOpen;
  };

  let apiKeyInput = '';
  let apiConnectionState: 'idle' | 'testing' | 'success' | 'error' = 'idle';
  let apiConnectionMessage = '';
  let isPromptSaved = true;

  // Dialog states
  let confirmRetranslateOpen = false;
  let confirmDeleteModelOpen = false;
  let confirmDeleteModelId = '';
  let confirmDeleteRuntimeOpen = false;
  let confirmClearHistoryOpen = false;
  let errorDialogOpen = false;
  let errorDialogTitle = '';
  let errorDialogMessage = '';
  let errorDialogDetail = '';
  let successDialogOpen = false;
  let successDialogTitle = '';
  let successDialogMessage = '';
  let inspectingMedia = false;

  const navItems: NavItem[] = [
    { key: 'translate', icon: WandSparkles },
    { key: 'instruction', icon: FileText },
    { key: 'render', icon: Video },
    { key: 'history', icon: HistoryIcon },
    { key: 'log', icon: FileText },
    { key: 'settings', icon: SettingsIcon }
  ];

  onMount(async () => {
    await bootstrapApp();
    apiKeyInput = await restoreSavedApiKey();
  });

  $: selectedWhisperModel =
    $whisperModels.find((model) => model.id === $activeJob.form.whisperModelId) ?? $whisperModels[0];
  $: canStart =
    !$busy &&
    !!$activeJob.form.inputPath &&
    ($activeJob.form.processingMode === 'gemini_direct' || selectedWhisperModel?.downloaded) &&
    !!apiKeyInput.trim();
  $: canRender = !!$activeJob.form.inputPath && $activeJob.translatedSegments.length > 0 && !$busy;
  $: renderIssues = ((): string[] => {
    const issues: string[] = [];
    if (!$activeJob.form.inputPath) issues.push('Chưa chọn file media.');
    if ($activeJob.translatedSegments.length === 0) issues.push('Chưa có subtitle để render.');
    if (!$settings.outputDirectory) issues.push('Chưa chọn thư mục output.');
    const { fontSize, outlineWidth, marginX, marginY, lineSpacing, textColor, outlineColor, fontFamily } = $activeJob.style;
    if (!fontFamily.trim()) issues.push('Font family không được để trống.');
    if (fontSize < 14 || fontSize > 80) issues.push('Font size phải từ 14–80px.');
    if (outlineWidth < 0 || outlineWidth > 8) issues.push('Outline width phải từ 0–8.');
    if (marginX < 0 || marginX > 160) issues.push('Horizontal margin phải từ 0–160.');
    if (marginY < 0 || marginY > 160) issues.push('Vertical margin phải từ 0–160.');
    if (lineSpacing < 0 || lineSpacing > 32) issues.push('Line spacing phải từ 0–32.');
    if (!/^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$/.test(textColor) && !textColor.startsWith('rgba') && !textColor.startsWith('rgb')) {
      issues.push('Text color không hợp lệ (dùng #rrggbb hoặc #rrggbbaa).');
    }
    if (outlineWidth > 0 && !/^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$/.test(outlineColor) && !outlineColor.startsWith('rgba') && !outlineColor.startsWith('rgb')) {
      issues.push('Outline color không hợp lệ.');
    }
    return issues;
  })();
  $: canExportSubtitles =
    $activeJob.status === 'completed' &&
    $activeJob.sourceSegments.length > 0 &&
    $activeJob.translatedSegments.length > 0;
  $: translatedSegmentCount = $activeJob.translatedSegments.length;
  $: usesWhisper = $activeJob.form.processingMode === 'whisper_translate';
  $: runtimeLabel =
    $runtimeCapabilities?.detectedAccelerators.join(', ') ||
    ($runtimeCapabilities?.hardwareAccelerationAvailable ? 'Hardware ready' : 'CPU only');
  $: runtimeInstalled =
    !!$runtimeCapabilities?.localFfmpegInstalled && !!$runtimeCapabilities?.localFfprobeInstalled;
  $: ffmpegReady =
    !!$runtimeCapabilities?.ffmpegAvailable || !!$runtimeCapabilities?.localFfmpegInstalled;
  $: whisperReady = true; // whisper-rs is bundled into the app
  $: runtimeBusy = $installingRuntime || $removingRuntime;
  $: showPipelineOverlay =
    ($busy || $cancelRequested) &&
    ['inspect', 'preprocess', 'transcribe', 'translate', 'render'].includes($activeJob.phase);
  $: {
    if ($activeJob.status === 'failed' && !$busy && $activeJob.phase === 'failed') {
      const latestLog = $processLogs.find((log) => log.jobId === $activeJob.id);
      showErrorDialog(
        'Pipeline thất bại',
        $activeJob.message || 'Có lỗi xảy ra trong quá trình xử lý.',
        latestLog?.detail ?? ''
      );
    }
  }
  $: {
    if ($activeJob.status === 'completed' && !$busy && $activeJob.phase === 'ready') {
      showSuccessDialog(
        'Hoàn thành!',
        'Pipeline đã chạy xong. Bạn có thể xem kết quả và render video.'
      );
    }
  }
  $: projectMenus = [
    {
      label: 'Project',
      items: [
        { label: 'Import project', shortcut: '⌘O', onselect: () => handleLoadProject() },
        { label: 'Export project', shortcut: '⌘S', onselect: () => handleSaveProject() }
      ]
    }
  ];

  function joinPath(base: string, filename: string): string {
    if (!base.trim()) {
      return filename;
    }

    return `${base.replace(/[\\/]+$/, '')}/${filename}`;
  }

  async function handleSelectMedia(): Promise<void> {
    const path = await openPathSelection(false, [
      {
        name: 'Media',
        extensions: ['mp4', 'mov', 'mkv', 'mp3', 'wav', 'm4a', 'aac']
      }
    ]);

    if (!path) {
      return;
    }

    inspectingMedia = true;
    try {
      await inspectSelectedMedia(path);
    } finally {
      inspectingMedia = false;
    }
  }

  async function handleChooseOutputDirectory(): Promise<void> {
    const path = await openPathSelection(true);
    if (!path) {
      return;
    }

    settings.update((current) => ({
      ...current,
      outputDirectory: path
    }));
  }

  async function handleStartPipeline(): Promise<void> {
    if ($activeJob.translatedSegments.length > 0) {
      confirmRetranslateOpen = true;
      return;
    }

    await startPipeline(apiKeyInput);
  }

  async function handleConfirmRetranslate(): Promise<void> {
    confirmRetranslateOpen = false;
    await startPipeline(apiKeyInput);
  }

  async function handleDeleteDownloadedModel(modelId: string): Promise<void> {
    confirmDeleteModelId = modelId;
    confirmDeleteModelOpen = true;
  }

  async function handleConfirmDeleteModel(): Promise<void> {
    confirmDeleteModelOpen = false;
    await deleteModel(confirmDeleteModelId);
    confirmDeleteModelId = '';
  }

  async function handleDeleteLocalRuntime(): Promise<void> {
    confirmDeleteRuntimeOpen = true;
  }

  async function handleConfirmDeleteRuntime(): Promise<void> {
    confirmDeleteRuntimeOpen = false;
    await removeManagedFfmpeg();
  }

  async function handleClearHistory(): Promise<void> {
    confirmClearHistoryOpen = true;
  }

  async function handleConfirmClearHistory(): Promise<void> {
    confirmClearHistoryOpen = false;
    clearHistory();
  }

  async function exportSubtitle(kind: 'source' | 'translated'): Promise<void> {
    const segments = kind === 'source' ? $activeJob.sourceSegments : $activeJob.translatedSegments;
    if (segments.length === 0) {
      return;
    }

    const content = segmentsToSrt(segments, kind === 'translated');
    const baseName = ($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '');
    const filename = `${baseName}.${kind === 'source' ? 'source' : 'vi'}.srt`;
    const path = await savePathSelection(filename);

    if (!path) {
      downloadTextFile(filename, content);
      return;
    }

    await writeTextContent(path, content);
  }

  async function exportAssSubtitle(): Promise<void> {
    const segments = $activeJob.translatedSegments;
    if (segments.length === 0) {
      return;
    }

    const baseName = ($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '');
    const filename = `${baseName}.ass`;
    const path = await savePathSelection(filename);

    if (!path) {
      return;
    }

    await exportAssSubtitleService({
      path,
      segments,
      style: $activeJob.style
    });
  }

  async function handleRenderVideo(): Promise<void> {
    if (!$activeJob.form.inputPath || $activeJob.translatedSegments.length === 0) {
      return;
    }

    const suggestedName = `${($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '')}.burned.mp4`;
    const outputPath = await savePathSelection(joinPath($settings.outputDirectory, suggestedName));

    if (!outputPath) {
      return;
    }

    await renderCurrentVideo(outputPath);
  }

  async function handleCancel(): Promise<void> {
    await requestCancellation();
  }

  async function handleSaveProject(): Promise<void> {
    const path = await savePathSelection(
      `${($activeJob.name || 'project').replace(/\.[^.]+$/, '')}.easyvietsub.json`
    );

    if (!path) {
      return;
    }

    await exportProject(path);
  }

  async function handleLoadProject(): Promise<void> {
    const path = await openPathSelection(false, [
      {
        name: 'EasyVietsub Project',
        extensions: ['json']
      }
    ]);

    if (!path) {
      return;
    }

    await importProject(path);
  }

  async function handleSaveApiKey(): Promise<void> {
    if (!apiKeyInput) {
      return;
    }

    await saveApiKey(apiKeyInput);
  }

  function showErrorDialog(title: string, message: string, detail?: string): void {
    errorDialogTitle = title;
    errorDialogMessage = message;
    errorDialogDetail = detail ?? '';
    errorDialogOpen = true;
  }

  function showSuccessDialog(title: string, message: string): void {
    successDialogTitle = title;
    successDialogMessage = message;
    successDialogOpen = true;
  }

  async function handleTestApiConnection(): Promise<void> {
    const key = apiKeyInput.trim();
    if (!key) {
      apiConnectionState = 'error';
      apiConnectionMessage = 'Nhập API key trước khi test.';
      return;
    }

    apiConnectionState = 'testing';
    apiConnectionMessage = 'Đang kiểm tra kết nối Gemini API...';

    try {
      const models = await fetchSupportedGeminiModels(key);
      let message = 'Kết nối thành công, nhưng key hiện không expose model nào nằm trong danh sách app support.';

      if (models.length > 0) {
        message = `Kết nối thành công (${models.length}/${FALLBACK_GEMINI_MODELS.length} model duoc app support).`;
      }

      apiConnectionState = 'success';
      apiConnectionMessage = message;
    } catch (error) {
      apiConnectionState = 'error';
      apiConnectionMessage = recordProcessError(
        'Gemini API connection',
        error,
        'Không thể kết nối Gemini API.'
      );
    }
  }

  function applyTimestampMask(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
    const input = event.currentTarget;
    let raw = input.value.replace(/[^\d]/g, '').slice(0, 9);
    let masked = '';

    for (let i = 0; i < raw.length; i++) {
      if (i === 2 || i === 4) masked += ':';
      if (i === 6) masked += ',';
      masked += raw[i];
    }

    input.value = masked;
  }

  function handleTimestampChange(
    segment: SubtitleSegment,
    field: 'startMs' | 'endMs',
    value: string
  ): void {
    const parsed = parseTimestamp(value);
    if (parsed === null) {
      return;
    }

    updateSubtitleSegment(segment.id, {
      [field]: parsed
    });
  }

  function handlePromptInput(html: string): void {
    updateForm('translationInstruction', html);
    isPromptSaved = false;
  }

  function handlePromptSave(html: string): void {
    updateForm('translationInstruction', html);
    saveTranslationInstruction(html);
    isPromptSaved = true;
  }

  function handlePromptRestore(): void {
    const defaultPrompt = DEFAULT_FORM.translationInstruction;
    updateForm('translationInstruction', defaultPrompt);
    saveTranslationInstruction(defaultPrompt);
    isPromptSaved = true;
  }

  function historyStatusVariant(
    status: 'draft' | 'running' | 'completed' | 'failed' | 'cancelled'
  ): 'default' | 'secondary' | 'outline' | 'destructive' {
    if (status === 'failed') {
      return 'destructive';
    }

    if (status === 'completed') {
      return 'default';
    }

    if (status === 'running') {
      return 'secondary';
    }

    return 'outline';
  }

  function colorPreviewStyle(color: string): string {
    return `background:${color || 'transparent'}`;
  }
</script>

<div class="min-h-screen bg-background">
  <Tabs.Root
    value={$currentView}
    onValueChange={(value) => setView(value as ViewKey)}
    class="mx-auto w-full max-w-350 px-3 py-3 sm:px-4"
  >
    <div class="sticky top-0 z-10 mb-3 rounded-lg border bg-card/95 p-2 backdrop-blur sm:p-2.5">
      <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-2">
          <div class="rounded-md bg-primary/10 p-1.5 text-primary">
            <Subtitles class="size-4" />
          </div>
          <h1 class="text-base font-semibold sm:text-lg">EasyVietsub</h1>
        </div>
        <Menubar menus={projectMenus} />
      </div>

      <div class="mt-2 overflow-x-auto">
        <Tabs.List class="inline-flex w-full min-w-max gap-1 bg-muted/30 p-1">
          {#each navItems as item}
            <Tabs.Trigger value={item.key} class="flex-1 px-2 py-1.5 text-xs sm:px-3 sm:text-sm">
              {@const Icon = item.icon}
              <Icon class="size-3.5 shrink-0" />
              <span class="hidden sm:inline">{VIEW_LABELS[item.key]}</span>
            </Tabs.Trigger>
          {/each}
        </Tabs.List>
      </div>
    </div>

    <main class="min-w-0">
      <Tabs.Content value="translate">
        <div class="space-y-2.5">
          <Collapsible.Root open class="group/input rounded-lg border bg-card">
            <div class="flex items-center justify-between gap-2 p-2.5 sm:p-3">
              <h2 class="text-base font-semibold sm:text-lg">Input & Model</h2>
              <div class="flex items-center gap-2">
                {#if $activeJob.form.processingMode === 'gemini_direct'}
                  <Badge variant="outline" class="border-amber-500/50 text-xs text-amber-700">
                    Experimental
                  </Badge>
                {/if}
                <Collapsible.Trigger class="rounded-md p-1 hover:bg-muted">
                  <ChevronDown class="size-4 transition-transform group-data-[state=open]/input:rotate-180" />
                </Collapsible.Trigger>
              </div>
            </div>
            <Collapsible.Content class="px-2.5 pb-2.5 sm:px-3 sm:pb-3">

            {#if $inspection}
              <div class="mb-2.5 grid grid-cols-3 gap-1.5">
                <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1.5 text-xs">
                  {#if $inspection.kind === 'video'}
                    <Video class="size-3" />
                  {:else}
                    <AudioLines class="size-3" />
                  {/if}
                  <span class="font-medium">{$inspection.kind}</span>
                </div>
                <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1.5 text-xs">
                  <BookAudio class="size-3" />
                  <span class="font-medium">{formatDuration($inspection.durationSeconds)}</span>
                </div>
                <div class="flex items-center gap-1.5 rounded-md border bg-muted/20 px-2 py-1.5 text-xs">
                  <Download class="size-3" />
                  <span class="font-medium">{$inspection.audioTracks.length} tracks</span>
                </div>
              </div>
            {/if}

            <div class="grid gap-4 xl:grid-cols-[minmax(0,1.12fr)_minmax(20rem,0.88fr)]">
              <div class="space-y-3">
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

                <div class="grid gap-3 md:grid-cols-2">
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
                        { value: 'whisper_translate', label: 'Whisper local + Gemini translate' },
                        { value: 'gemini_direct', label: 'Gemini direct audio (experimental)' }
                      ]}
                      onchange={(event) =>
                        updateForm(
                          'processingMode',
                          event.currentTarget.value as typeof DEFAULT_FORM.processingMode
                        )
                      }
                    />
                  </Field>
                </div>

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
                        label: `Track ${track.index} • ${track.language || 'unknown'} • ${track.codec}`
                      }))}
                      onchange={(event) => updateForm('selectedAudioTrack', Number(event.currentTarget.value))}
                    />
                  </Field>
                {/if}
              </div>

              <div class="space-y-3">
                {#if usesWhisper}
                  <div class="rounded-xl border bg-muted/20 p-3">
                    <div class="grid gap-3 md:grid-cols-[minmax(0,12rem)_minmax(0,1fr)]">
                      <Field label="Compute mode">
                        <Select
                          value={$activeJob.form.computeMode}
                          options={[
                            { value: 'auto', label: 'Auto' },
                            { value: 'cpu', label: 'CPU only' },
                            { value: 'hardware', label: 'Hardware acceleration' }
                          ]}
                          onchange={(event) =>
                            updateForm('computeMode', event.currentTarget.value as typeof DEFAULT_FORM.computeMode)
                          }
                        />
                      </Field>

                      <Field label="Whisper model">
                        <div class="space-y-2">
                          <Select
                            value={$activeJob.form.whisperModelId}
                            disabled={$busy}
                            options={$whisperModels.map((model) => ({
                              value: model.id,
                              label: `${model.label} • ${formatBytes(model.sizeBytes)}`
                            }))}
                            onchange={(event) => updateForm('whisperModelId', event.currentTarget.value)}
                          />

                          {#if selectedWhisperModel}
                            <div class="flex items-center justify-between gap-2 rounded-md border bg-background/70 px-3 py-2">
                              <div class="min-w-0">
                                <p class="truncate text-sm font-medium text-foreground">
                                  {selectedWhisperModel.label}
                                </p>
                              </div>
                              <div class="flex items-center gap-2">
                          <Button
                            variant={selectedWhisperModel.downloaded ? 'secondary' : 'outline'}
                            size="sm"
                            disabled={$busy || selectedWhisperModel.downloaded}
                            onclick={() => downloadModel(selectedWhisperModel.id)}
                          >
                            {#if $downloadingModelId === selectedWhisperModel.id}
                              <CircleProgress class="size-4" />
                              {$downloadingModelProgress}%
                            {:else}
                              {selectedWhisperModel.downloaded ? 'Đã tải' : 'Tải model'}
                            {/if}
                          </Button>
                                {#if selectedWhisperModel.downloaded}
                                  <Button
                                    variant="outline"
                                    size="icon"
                                    title="Delete downloaded model"
                                    ariaLabel="Delete downloaded model"
                                    disabled={$busy}
                                    onclick={() => handleDeleteDownloadedModel(selectedWhisperModel.id)}
                                  >
                                    <Trash2 class="size-4" />
                                  </Button>
                                {/if}
                              </div>
                            </div>
                          {/if}
                        </div>
                      </Field>
                    </div>
                  </div>
                {/if}

                <div class="rounded-xl border bg-muted/20 px-3 py-3">
                  <div class="flex items-center justify-between gap-3">
                    <p class="truncate text-sm text-muted-foreground">{$statusMessage}</p>
                  </div>
                </div>

                {#if !ffmpegReady}
                  <div class="flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2.5 text-sm text-yellow-700 dark:text-yellow-400">
                    <AlertTriangle class="mt-0.5 size-4 shrink-0" />
                    <span>Cần cài đặt FFmpeg. Vào tab <strong>Cài đặt</strong> để cài đặt.</span>
                  </div>
                {/if}

                {#if usesWhisper && !whisperReady}
                  <div class="flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2.5 text-sm text-yellow-700 dark:text-yellow-400">
                    <AlertTriangle class="mt-0.5 size-4 shrink-0" />
                    <span>Cần tải Whisper model. Vào tab <strong>Cài đặt</strong> để tải.</span>
                  </div>
                {/if}

                <Button class="w-full sm:w-auto" disabled={!canStart || !ffmpegReady || (usesWhisper && !whisperReady)} onclick={handleStartPipeline}>
                  <Play class="size-4" />
                  Bắt đầu
                </Button>
              </div>
            </div>
            </Collapsible.Content>
          </Collapsible.Root>

          <Collapsible.Root open class="group/editor rounded-lg border bg-card">
            <div class="flex items-center justify-between gap-2 p-2.5 sm:p-3">
              <h2 class="text-base font-semibold sm:text-lg">Subtitle Editor</h2>
              <div class="flex items-center gap-2">
                {#if canExportSubtitles}
                  <div class="flex gap-1.5">
                    <Button variant="outline" size="sm" onclick={() => exportSubtitle('source')} class="text-xs">
                      <Languages class="size-3.5" />
                      <span class="hidden sm:inline">Source</span>
                    </Button>
                    <Button variant="outline" size="sm" onclick={() => exportSubtitle('translated')} class="text-xs">
                      <Languages class="size-3.5" />
                      <span class="hidden sm:inline">Vietnamese</span>
                    </Button>
                    <Button variant="outline" size="sm" onclick={exportAssSubtitle} class="text-xs">
                      <FileText class="size-3.5" />
                      <span class="hidden sm:inline">ASS</span>
                    </Button>
                  </div>
                {/if}
                <Collapsible.Trigger class="rounded-md p-1 hover:bg-muted">
                  <ChevronDown class="size-4 transition-transform group-data-[state=open]/editor:rotate-180" />
                </Collapsible.Trigger>
              </div>
            </div>
            <Collapsible.Content class="px-2.5 pb-2.5 sm:px-3 sm:pb-3">

            {#if $validationErrors.length > 0}
              <div class="mb-2.5 rounded-lg border border-destructive/30 bg-destructive/5 p-2 text-xs text-destructive">
                {#each $validationErrors as error}
                  <p>{error}</p>
                {/each}
              </div>
            {/if}

            <div class="soft-scrollbar max-h-128 space-y-1.5 overflow-auto pr-1">
              {#if translatedSegmentCount === 0}
              <div class="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
                Chạy pipeline để tạo subtitle.
              </div>
              {:else}
                {#each $activeJob.translatedSegments as segment, index}
                  <div class="rounded-lg border bg-background/70">
                    <!-- header row: index + timestamps + duration -->
                    <div class="flex items-center gap-2 border-b px-2.5 py-1.5">
                      <span class="w-6 shrink-0 text-center text-md font-semibold text-muted-foreground">
                        {index + 1}
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

                    <!-- translated text only -->
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
      </Tabs.Content>

      <Tabs.Content value="instruction">
          <Card class="border bg-card p-2.5 sm:p-3">
            <h2 class="mb-2 text-base font-semibold sm:text-lg">Prompt cho AI</h2>
            
            <TiptapEditor
              value={$activeJob.form.translationInstruction}
              oninput={handlePromptInput}
              onsave={handlePromptSave}
              onrestore={handlePromptRestore}
              isSaved={isPromptSaved}
            />
        </Card>
      </Tabs.Content>

      <Tabs.Content value="render">
        <div class="space-y-2.5">
          <Card class="border bg-card p-2.5 sm:p-3">
            <h2 class="mb-2.5 text-base font-semibold sm:text-lg">Subtitle Styling & Render</h2>

            <div class="grid gap-3 lg:grid-cols-2">
              <div class="rounded-lg border bg-background/60 p-2">
                <PreviewCanvas 
                  style={$activeJob.style} 
                  segments={$activeJob.translatedSegments}
                  mediaPath={$activeJob.form.inputPath || null}
                  mediaKind={$activeJob.form.inputKind}
                />
              </div>

              <div class="space-y-3">
            <div class="rounded-lg border bg-muted/20 p-2.5">
              <p class="mb-2 text-sm font-medium">Render video với phụ đề</p>

              <div class="space-y-2">
                    <Field label="Output directory">
                      <div class="flex items-center gap-1.5">
                        <Input value={$settings.outputDirectory} disabled class="flex-1 text-xs" />
                        <Button
                          variant="outline"
                          size="icon"
                          title="Browse output"
                          ariaLabel="Browse output"
                          onclick={handleChooseOutputDirectory}
                        >
                          <FolderOpen class="size-3.5" />
                        </Button>
                      </div>
                    </Field>

                <Button class="w-full" disabled={!canRender || renderIssues.length > 0} onclick={handleRenderVideo} size="sm">
                  <Video class="size-3.5" />
                  Render
                </Button>

                    {#if renderIssues.length > 0}
                      <div class="rounded-md border border-amber-500/30 bg-amber-50/10 p-2 text-xs text-amber-700 dark:text-amber-400 space-y-0.5">
                        {#each renderIssues as issue}
                          <p>• {issue}</p>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>

            <div class="rounded-lg border bg-muted/20 p-2.5">
              <p class="mb-2 text-sm font-medium">Font & Kiểu chữ</p>

              <div class="space-y-2">
                    <div class="grid grid-cols-2 gap-1.5">
                      <Field label="Font size">
                        <Input
                          type="number"
                          value={$activeJob.style.fontSize}
                          min={14}
                          max={80}
                          onchange={(event) => updateStyle('fontSize', Number(event.currentTarget.value))}
                          class="h-8 text-xs"
                        />
                      </Field>
                      <Field label="Line spacing">
                        <Input
                          type="number"
                          value={$activeJob.style.lineSpacing}
                          min={0}
                          max={32}
                          onchange={(event) => updateStyle('lineSpacing', Number(event.currentTarget.value))}
                          class="h-8 text-xs"
                        />
                      </Field>
                    </div>

                    <div class="grid grid-cols-2 gap-1.5">
                      <Field label="Text color">
                        <div class="flex items-center gap-1.5">
                          <span
                            class="size-7 shrink-0 rounded border"
                            style={colorPreviewStyle($activeJob.style.textColor)}
                          ></span>
                          <Input
                            value={$activeJob.style.textColor}
                            oninput={(event) => updateStyle('textColor', event.currentTarget.value)}
                            class="h-8 text-xs"
                          />
                        </div>
                      </Field>
                      <Field label="Outline color">
                        <div class="flex items-center gap-1.5">
                          <span
                            class="size-7 shrink-0 rounded border"
                            style={colorPreviewStyle($activeJob.style.outlineColor)}
                          ></span>
                          <Input
                            value={$activeJob.style.outlineColor}
                            oninput={(event) => updateStyle('outlineColor', event.currentTarget.value)}
                            class="h-8 text-xs"
                          />
                        </div>
                      </Field>
                    </div>

                    <div class="grid grid-cols-2 gap-1.5">
                      <Field label="Position">
                        <Select
                          value={$activeJob.style.position}
                          options={[
                            { value: 'bottom', label: 'Bottom' },
                            { value: 'center', label: 'Center' },
                            { value: 'top', label: 'Top' }
                          ]}
                          onchange={(event) =>
                            updateStyle('position', event.currentTarget.value as 'bottom' | 'center' | 'top')}
                          class="h-8 text-xs"
                        />
                      </Field>
                      <Field label="Outline width">
                        <Input
                          type="number"
                          value={$activeJob.style.outlineWidth}
                          min={0}
                          max={8}
                          onchange={(event) => updateStyle('outlineWidth', Number(event.currentTarget.value))}
                          class="h-8 text-xs"
                        />
                      </Field>
                    </div>

                    <div class="grid grid-cols-2 gap-1.5">
                      <Field label="Horizontal margin">
                        <Input
                          type="number"
                          value={$activeJob.style.marginX}
                          min={0}
                          max={160}
                          onchange={(event) => updateStyle('marginX', Number(event.currentTarget.value))}
                          class="h-8 text-xs"
                        />
                      </Field>
                      <Field label="Vertical margin">
                        <Input
                          type="number"
                          value={$activeJob.style.marginY}
                          min={0}
                          max={160}
                          onchange={(event) => updateStyle('marginY', Number(event.currentTarget.value))}
                          class="h-8 text-xs"
                        />
                      </Field>
                    </div>

                    <div class="flex gap-1.5">
                      <Button
                        variant={$activeJob.style.bold ? 'default' : 'outline'}
                        size="sm"
                        onclick={() => updateStyle('bold', !$activeJob.style.bold)}
                        class="flex-1 text-xs"
                      >
                        Bold
                      </Button>
                      <Button
                        variant={$activeJob.style.italic ? 'default' : 'outline'}
                        size="sm"
                        onclick={() => updateStyle('italic', !$activeJob.style.italic)}
                        class="flex-1 text-xs"
                      >
                        Italic
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </Card>

        </div>
      </Tabs.Content>

      <Tabs.Content value="history">
        <Card class="border bg-card p-3">
          <div class="mb-3 flex items-center justify-between gap-2">
            <h2 class="text-base font-semibold sm:text-lg">History ({$history.length})</h2>
            {#if $history.length > 0}
              <Button variant="outline" size="sm" onclick={handleClearHistory} class="text-xs">
                <Trash2 class="size-3.5" />
                <span class="hidden sm:inline">Xóa tất cả</span>
              </Button>
            {/if}
          </div>

          {#if $history.length === 0}
            <div class="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
              Chưa có job nào.
            </div>
          {:else}
            <div class="space-y-1.5">
              {#each $history as item}
                <div class="flex w-full items-center gap-2 rounded-lg border bg-background/70 px-3 py-2 text-sm transition hover:bg-accent/40">
                  <button
                    class="flex flex-1 items-center justify-between gap-2 text-left"
                    onclick={() => {
                      setView('translate');
                      loadJobFromHistory(item);
                    }}
                    type="button"
                  >
                    <div class="min-w-0 flex-1">
                      <p class="truncate font-medium">{item.name}</p>
                      <p class="mt-0.5 text-xs text-muted-foreground">
                        {new Date(item.updatedAt).toLocaleString('vi-VN', { dateStyle: 'short', timeStyle: 'short' })}
                      </p>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs text-muted-foreground">{item.translatedSegments.length} segs</span>
                      <Badge variant={item.status === 'completed' ? 'default' : 'outline'} class="text-xs">
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
        </Card>
      </Tabs.Content>

      <Tabs.Content value="log">
        <Card class="border bg-card p-3">
          <div class="mb-3 flex items-center justify-between gap-2">
            <h2 class="text-base font-semibold sm:text-lg">Error Logs ({$processLogs.length})</h2>
          </div>

          {#if $processLogs.length === 0}
            <div class="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
              Chưa có lỗi nào.
            </div>
          {:else}
            <div class="space-y-1.5">
              {#each $processLogs as item}
                <div class="rounded-lg border border-destructive/20 bg-destructive/5 p-2.5">
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
        </Card>
      </Tabs.Content>

      <Tabs.Content value="settings">
        <div class="grid gap-2.5 lg:grid-cols-3">
          <!-- Column 1: API Key & Settings -->
          <Card class="border bg-card p-2.5 sm:p-3">
            <h2 class="mb-2.5 text-base font-semibold sm:text-lg">Cài đặt API</h2>

            <div class="space-y-3">
              <div class="rounded-lg border bg-muted/20 p-2.5">
                <div class="mb-2 flex items-center gap-1.5">
                  <KeyRound class="size-3.5" />
                  <h3 class="text-sm font-medium">Gemini API key</h3>
                </div>
                <div class="space-y-2">
                  <Field label="API key">
                    <Input
                      value={apiKeyInput}
                      placeholder="AIza..."
                      oninput={(event) => (apiKeyInput = event.currentTarget.value)}
                      class="h-8 text-xs"
                    />
                  </Field>
                  <div class="flex gap-1.5">
                    <Button onclick={handleSaveApiKey} size="sm" class="flex-1 text-xs">Lưu</Button>
                    <Button
                      variant="outline"
                      disabled={apiConnectionState === 'testing'}
                      onclick={handleTestApiConnection}
                      size="sm"
                      class="flex-1 text-xs"
                    >
                      {#if apiConnectionState === 'testing'}
                        <CircleProgress class="size-3.5" />
                      {/if}
                      Test
                    </Button>
                  </div>
                  {#if apiConnectionState !== 'idle'}
                    <div
                      class={`rounded-md border px-2 py-1.5 text-xs ${
                        apiConnectionState === 'error'
                          ? 'border-destructive/30 bg-destructive/5 text-destructive'
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

          <!-- Column 2: Local FFmpeg -->
          <Card class="border bg-card p-2.5 sm:p-3">
            <div class="mb-2.5 flex items-center justify-between gap-2">
              <h2 class="text-base font-semibold sm:text-lg">FFmpeg</h2>
              <div
                class={`rounded-full border px-2 py-0.5 text-xs font-medium ${runtimeInstalled ? 'border-green-500/40 bg-green-500/10 text-green-600 dark:text-green-400' : 'border-border text-muted-foreground'}`}
              >
                {runtimeInstalled ? 'Đã cài' : 'Chưa cài'}
              </div>
            </div>

            <div class="space-y-2">

              <div class="rounded-md border bg-muted/20 px-2 py-1.5 text-xs">
                {runtimeLabel}
              </div>

              <Button
                class="w-full"
                variant={runtimeInstalled ? 'outline' : 'default'}
                disabled={runtimeBusy}
                onclick={runtimeInstalled ? handleDeleteLocalRuntime : installManagedFfmpeg}
                size="sm"
              >
                {#if $installingRuntime}
                  <CircleProgress class="size-3.5" />
                  Đang cài {$runtimeDownloadProgress}%
                {:else if $removingRuntime}
                  <CircleProgress class="size-3.5" />
                  Đang xóa
                {:else if runtimeInstalled}
                  <Trash2 class="size-3.5" />
                  Xóa
                {:else}
                  <Download class="size-3.5" />
                  Cài đặt
                {/if}
              </Button>

              {#if $installingRuntime || $runtimeDownloadMessage}
                <div class="rounded-md border bg-background/70 px-2 py-1.5 text-xs">
                  {$runtimeDownloadMessage || 'Đang chuẩn bị...'}
                </div>
              {/if}
            </div>
          </Card>

          <!-- Column 3: Whisper -->
          <Card class="border bg-card p-2.5 sm:p-3">
            <div class="mb-2.5 flex items-center justify-between gap-2">
              <h2 class="text-base font-semibold sm:text-lg">Whisper</h2>
              <div
                class="rounded-full border border-green-500/40 bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-600 dark:text-green-400"
              >
                Bundled
              </div>
            </div>

            <div class="space-y-2">
              <!-- Whisper Models -->
              <div class="rounded-lg border bg-muted/20 p-2.5">
                <p class="mb-2 text-xs font-medium">Models</p>
                <p class="mb-2 text-xs text-muted-foreground">
                  Tải model Whisper để chuyển giọng nói thành văn bản
                </p>
                <div class="space-y-1.5">
                  {#each $whisperModels as model}
                    <div class="flex items-center justify-between gap-2 rounded-md border bg-background/70 px-2.5 py-1.5">
                      <div class="min-w-0">
                        <p class="text-xs font-medium">{model.label}</p>
                        <p class="text-xs text-muted-foreground">{formatBytes(model.sizeBytes)}</p>
                      </div>
                      {#if $downloadingModelId === model.id}
                        <div class="flex shrink-0 flex-col items-center gap-0.5">
                          <CircleProgress class="size-5" />
                          {#if $downloadingModelProgress > 0 && $downloadingModelProgress < 100}
                            <span class="text-[10px] text-muted-foreground">{$downloadingModelProgress}%</span>
                          {/if}
                        </div>
                      {:else if $deletingModelId === model.id}
                        <div class="flex shrink-0 items-center gap-1">
                          <CircleProgress class="size-4" />
                          <span class="text-[10px] text-muted-foreground">Đang xóa...</span>
                        </div>
                      {:else if model.downloaded}
                        <Button variant="ghost" size="sm" onclick={() => deleteModel(model.id)} class="text-xs">
                          <Trash2 class="size-3.5" />
                        </Button>
                      {:else}
                        <Button variant="outline" size="sm" onclick={() => downloadModel(model.id)} class="text-xs">
                          <Download class="size-3.5" />
                        </Button>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          </Card>
        </div>
      </Tabs.Content>
    </main>
  </Tabs.Root>

  {#if showPipelineOverlay}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-background/60 px-4 backdrop-blur-md">
      <Card class="w-full max-w-sm border bg-card/90 p-5 shadow-xl">
        <div class="flex flex-col items-center text-center">
          <CircleProgress class="text-primary" size={88} />
          <p class="mt-4 text-sm font-medium">{$statusMessage}</p>

          <Button class="mt-5 w-full" variant="outline" disabled={$cancelRequested} onclick={handleCancel}>
            <Square class="size-4" />
            {$cancelRequested ? 'Đang hủy...' : 'Hủy'}
          </Button>
        </div>
      </Card>
    </div>
  {/if}

  <!-- Re-translate confirmation dialog -->
  <AlertDialog.Root bind:open={confirmRetranslateOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Xác nhận chạy lại</AlertDialog.Title>
        <AlertDialog.Description>
          Chạy lại sẽ xóa toàn bộ kết quả hiện tại. Bạn có muốn tiếp tục?
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel>
          <Button variant="outline" size="sm">Hủy</Button>
        </AlertDialog.Cancel>
        <AlertDialog.Action>
          <Button variant="destructive" size="sm" onclick={handleConfirmRetranslate}>Tiếp tục</Button>
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <!-- Delete whisper model confirmation dialog -->
  <AlertDialog.Root bind:open={confirmDeleteModelOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Xóa Whisper model</AlertDialog.Title>
        <AlertDialog.Description>
          Xóa Whisper model <strong>{confirmDeleteModelId}</strong> khỏi local cache?
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel>
          <Button variant="outline" size="sm">Hủy</Button>
        </AlertDialog.Cancel>
        <AlertDialog.Action>
          <Button variant="destructive" size="sm" onclick={handleConfirmDeleteModel}>Xóa</Button>
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <!-- Delete FFmpeg runtime confirmation dialog -->
  <AlertDialog.Root bind:open={confirmDeleteRuntimeOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Xóa FFmpeg local</AlertDialog.Title>
        <AlertDialog.Description>
          Xóa FFmpeg local đã cài khỏi app? Bạn có thể cài lại bất kỳ lúc nào.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel>
          <Button variant="outline" size="sm">Hủy</Button>
        </AlertDialog.Cancel>
        <AlertDialog.Action>
          <Button variant="destructive" size="sm" onclick={handleConfirmDeleteRuntime}>Xóa</Button>
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <!-- Clear history confirmation dialog -->
  <AlertDialog.Root bind:open={confirmClearHistoryOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Xóa tất cả history</AlertDialog.Title>
        <AlertDialog.Description>
          Xóa tất cả {$history.length} job khỏi history? Hành động này không thể hoàn tác.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel>
          <Button variant="outline" size="sm">Hủy</Button>
        </AlertDialog.Cancel>
        <AlertDialog.Action>
          <Button variant="destructive" size="sm" onclick={handleConfirmClearHistory}>Xóa tất cả</Button>
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <!-- Error dialog -->
  <Dialog.Root bind:open={errorDialogOpen}>
    <Dialog.Content>
      <Dialog.Header>
        <Dialog.Title class="text-destructive">{errorDialogTitle}</Dialog.Title>
        <Dialog.Description>{errorDialogMessage}</Dialog.Description>
      </Dialog.Header>
      {#if errorDialogDetail}
        <pre class="mt-2 max-h-64 overflow-auto whitespace-pre-wrap wrap-break-word rounded-md border bg-muted/30 px-3 py-2 text-xs">{errorDialogDetail}</pre>
      {/if}
      <Dialog.Footer>
        <Dialog.Close>
          <Button variant="outline" size="sm" onclick={() => { errorDialogOpen = false; setView('log'); }}>Xem logs</Button>
        </Dialog.Close>
        <Dialog.Close>
          <Button size="sm" onclick={() => { errorDialogOpen = false; }}>Đóng</Button>
        </Dialog.Close>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>

  <!-- Success dialog -->
  <Dialog.Root bind:open={successDialogOpen}>
    <Dialog.Content>
      <Dialog.Header>
        <Dialog.Title class="text-green-600 dark:text-green-400">{successDialogTitle}</Dialog.Title>
        <Dialog.Description>{successDialogMessage}</Dialog.Description>
      </Dialog.Header>
      <Dialog.Footer>
        <Dialog.Close>
          <Button size="sm" onclick={() => { successDialogOpen = false; }}>Đóng</Button>
        </Dialog.Close>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
</div>
