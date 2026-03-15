<script lang="ts">
  import Sidebar from '$lib/components/layout/sidebar.svelte';
  import TranslateView from '$lib/components/views/translate-view.svelte';
  import InstructionView from '$lib/components/views/instruction-view.svelte';
  import RenderView from '$lib/components/views/render-view.svelte';
  import HistoryView from '$lib/components/views/history-view.svelte';
  import LogView from '$lib/components/views/log-view.svelte';
  import SettingsView from '$lib/components/views/settings-view.svelte';
  import AppDialogs from '$lib/components/dialogs/app-dialogs.svelte';
  import PipelineOverlay from '$lib/components/dialogs/pipeline-overlay.svelte';
  import Menubar from '$lib/components/ui/menubar.svelte';
  import {
    activeJob,
    bootstrapApp,
    busy,
    cancelRequested,
    canRedo,
    canUndo,
    clearHistory,
    cleanupEventListeners,
    currentView,
    deleteModel,
    exportProject,
    importProject,
    inspectSelectedMedia,
    processLogs,
    redo,
    removeManagedFfmpeg,
    requestCancellation,
    restoreSavedApiKey,
    retranslateOnly,
    saveApiKey,
    startPipeline,
    undo
  } from '$lib/stores/app-store';
  import { fetchSupportedGeminiModels, openPathSelection, savePathSelection } from '$lib/services/tauri';
  import { PIPELINE_TIMER_INTERVAL_MS } from '$lib/constants';
  import { onDestroy, onMount } from 'svelte';

  // Local state
  let apiKeyInput = $state('');
  let apiConnectionState: 'idle' | 'testing' | 'success' | 'error' = $state('idle');
  let apiConnectionMessage = $state('');

  // Dialog states
  let confirmRetranslateOpen = $state(false);
  let confirmRetranslateOnlyOpen = $state(false);
  let confirmDeleteModelOpen = $state(false);
  let confirmDeleteModelId = $state('');
  let confirmDeleteRuntimeOpen = $state(false);
  let confirmClearHistoryOpen = $state(false);
  let errorDialogOpen = $state(false);
  let errorDialogTitle = $state('');
  let errorDialogMessage = $state('');
  let errorDialogDetail = $state('');
  let successDialogOpen = $state(false);
  let successDialogTitle = $state('');
  let successDialogMessage = $state('');
  let lastCompletedJobId = $state('');
  let lastFailedJobId = $state('');

  // Pipeline timer
  let pipelineElapsedSeconds = $state(0);
  let pipelineTimerInterval: ReturnType<typeof setInterval> | null = null;

  function startPipelineTimer() {
    stopPipelineTimer();
    pipelineElapsedSeconds = 0;
    pipelineTimerInterval = setInterval(() => {
      pipelineElapsedSeconds += 1;
    }, PIPELINE_TIMER_INTERVAL_MS);
  }

  function stopPipelineTimer() {
    if (pipelineTimerInterval) {
      clearInterval(pipelineTimerInterval);
      pipelineTimerInterval = null;
    }
  }

  let showPipelineOverlay = $derived(
    ($busy || $cancelRequested) &&
    ['inspect', 'preprocess', 'transcribe', 'translate', 'render'].includes($activeJob.phase)
  );

  $effect(() => {
    if (showPipelineOverlay) {
      if (!pipelineTimerInterval) startPipelineTimer();
    } else {
      stopPipelineTimer();
    }
  });

  onDestroy(() => {
    stopPipelineTimer();
    cleanupEventListeners();
  });

  onMount(async () => {
    await bootstrapApp();
    apiKeyInput = restoreSavedApiKey();
  });

  // Setup global keyboard shortcuts
  onMount(() => {
    const handleKeydown = (event: KeyboardEvent) => {
      // Check for Cmd (macOS) or Ctrl (Windows/Linux)
      const modifier = event.metaKey || event.ctrlKey;
      
      if (modifier && event.key === 's') {
        event.preventDefault();
        handleSaveProject();
      } else if (modifier && event.key === 'o') {
        event.preventDefault();
        handleLoadProject();
      } else if (modifier && event.key === ',') {
        event.preventDefault();
        currentView.set('settings');
      } else if (modifier && event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        if ($canUndo) undo();
      } else if (modifier && (event.key === 'Z' || (event.key === 'z' && event.shiftKey))) {
        event.preventDefault();
        if ($canRedo) redo();
      } else if (modifier && event.key === 'y') {
        event.preventDefault();
        if ($canRedo) redo();
      } else if (modifier && event.key === '1') {
        event.preventDefault();
        currentView.set('translate');
      } else if (modifier && event.key === '2') {
        event.preventDefault();
        currentView.set('instruction');
      } else if (modifier && event.key === '3') {
        event.preventDefault();
        currentView.set('render');
      } else if (modifier && event.key === '4') {
        event.preventDefault();
        currentView.set('history');
      } else if (modifier && event.key === '5') {
        event.preventDefault();
        currentView.set('log');
      } else if (event.key === 'Escape' && showPipelineOverlay) {
        event.preventDefault();
        handleCancel();
      }
    };
    
    window.addEventListener('keydown', handleKeydown);
    
    return () => {
      window.removeEventListener('keydown', handleKeydown);
    };
  });

  // Auto-show error dialog on failure
  $effect(() => {
    if ($activeJob.status === 'failed' && !$busy && $activeJob.phase === 'failed' && $activeJob.id !== lastFailedJobId) {
      lastFailedJobId = $activeJob.id;
      const latestLog = $processLogs.find((log) => log.jobId === $activeJob.id);
      showErrorDialog(
        'Pipeline thất bại',
        $activeJob.message || 'Có lỗi xảy ra trong quá trình xử lý.',
        latestLog?.detail ?? ''
      );
    }
  });


  // Auto-show success dialog on completion
  $effect(() => {
    const shouldShowDialog = 
      ($activeJob.status === 'completed' || $activeJob.status === 'partial') && 
      !$busy && 
      $activeJob.phase === 'ready' &&
      $activeJob.id !== lastCompletedJobId;

    if (shouldShowDialog) {
      lastCompletedJobId = $activeJob.id;
      
      if ($activeJob.status === 'partial') {
        showSuccessDialog(
          'Hoàn thành (một phần)',
          'Whisper đã transcribe xong nhưng Gemini dịch thất bại. Subtitle ngôn ngữ gốc đã được giữ lại.'
        );
      } else {
        showSuccessDialog(
          'Hoàn thành!',
          'Pipeline đã chạy xong. Bạn có thể xem kết quả và render video.'
        );
      }
    }
  });

  let projectMenus = $derived([
    {
      label: 'Project',
      items: [
        { label: 'Import project', shortcut: '⌘O', onselect: () => handleLoadProject() },
        { label: 'Export project', shortcut: '⌘S', onselect: () => handleSaveProject() }
      ]
    }
  ]);

  // Handlers
  async function handleConfirmRetranslate(): Promise<void> {
    confirmRetranslateOpen = false;
    await startPipeline(apiKeyInput);
  }

  async function handleConfirmRetranslateOnly(): Promise<void> {
    confirmRetranslateOnlyOpen = false;
    lastCompletedJobId = '';
    await retranslateOnly(apiKeyInput);
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

  async function handleCancel(): Promise<void> {
    await requestCancellation();
  }

  async function handleSaveProject(): Promise<void> {
    const path = await savePathSelection(
      `${($activeJob.name || 'project').replace(/\.[^.]+$/, '')}.easyvietsub.json`
    );
    if (!path) return;
    await exportProject(path);
  }

  async function handleLoadProject(): Promise<void> {
    const path = await openPathSelection(false, [
      {
        name: 'EasyVietsub Project',
        extensions: ['json']
      }
    ]);
    if (!path) return;
    await importProject(path);
  }

  async function handleSaveApiKey(): Promise<void> {
    if (!apiKeyInput) return;
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
    apiConnectionMessage = 'Đang kết nối...';

    try {
      await fetchSupportedGeminiModels(key);
      apiConnectionState = 'success';
      apiConnectionMessage = 'Kết nối thành công!';
    } catch (err) {
      apiConnectionState = 'error';
      const msg = err instanceof Error ? err.message : 'Lỗi không xác định';
      apiConnectionMessage = `Kết nối thất bại: ${msg}`;
    }
  }

  function handleLoadJobFromHistory(jobId: string): void {
    lastCompletedJobId = jobId;
  }
</script>

<div class="min-h-screen bg-background">
  <Sidebar />

  <!-- Main content area with left padding for fixed sidebar -->
  <div class="ml-48">
    <!-- Top header -->
    <header class="sticky top-0 z-30 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div class="flex h-13 items-center gap-3 px-4">
        <Menubar menus={projectMenus} />
      </div>
    </header>

    <!-- Main view content -->
    <main class="max-w-350 p-4">
      {#if $currentView === 'translate'}
        <TranslateView
          bind:apiKeyInput
          onConfirmRetranslate={() => { confirmRetranslateOpen = true; }}
          onConfirmRetranslateOnly={() => { confirmRetranslateOnlyOpen = true; }}
          onDeleteModel={handleDeleteDownloadedModel}
        />
      {:else if $currentView === 'instruction'}
        <InstructionView />
      {:else if $currentView === 'render'}
        <RenderView />
      {:else if $currentView === 'history'}
        <HistoryView
          onClearHistory={handleClearHistory}
          onLoadJob={handleLoadJobFromHistory}
        />
      {:else if $currentView === 'log'}
        <LogView />
      {:else if $currentView === 'settings'}
        <SettingsView
          bind:apiKeyInput
          {apiConnectionState}
          {apiConnectionMessage}
          onSaveApiKey={handleSaveApiKey}
          onTestApiConnection={handleTestApiConnection}
          onDeleteModel={handleDeleteDownloadedModel}
          onDeleteRuntime={handleDeleteLocalRuntime}
        />
      {/if}
    </main>
  </div>

  {#if showPipelineOverlay}
    <PipelineOverlay
      {pipelineElapsedSeconds}
      onCancel={handleCancel}
    />
  {/if}

  <AppDialogs
    bind:confirmRetranslateOpen
    bind:confirmRetranslateOnlyOpen
    bind:confirmDeleteModelOpen
    {confirmDeleteModelId}
    bind:confirmDeleteRuntimeOpen
    bind:confirmClearHistoryOpen
    bind:errorDialogOpen
    {errorDialogTitle}
    {errorDialogMessage}
    {errorDialogDetail}
    bind:successDialogOpen
    {successDialogTitle}
    {successDialogMessage}
    onConfirmRetranslate={handleConfirmRetranslate}
    onConfirmRetranslateOnly={handleConfirmRetranslateOnly}
    onConfirmDeleteModel={handleConfirmDeleteModel}
    onConfirmDeleteRuntime={handleConfirmDeleteRuntime}
    onConfirmClearHistory={handleConfirmClearHistory}
  />
</div>
