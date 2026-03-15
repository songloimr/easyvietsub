import {
  APP_STORAGE_KEY,
  DEFAULT_FORM,
  DEFAULT_SETTINGS,
  DEFAULT_STYLE,
  MAX_HISTORY_ENTRIES,
  MAX_LOG_ENTRIES,
  MODEL_DOWNLOAD_PROGRESS_THROTTLE_MS,
  PERSIST_DEBOUNCE_MS,
  PIPELINE_TIMER_INTERVAL_MS,
  PROGRESS_COMPLETE,
  PROJECT_SCHEMA_VERSION,
  SUBTITLE_UPDATE_DEBOUNCE_MS,
  WHISPER_MODELS
} from '$lib/constants';
import { formatError, getErrorDetails, getErrorMessage } from '$lib/errors';
import {
  cancelPipelineProcess,
  detectRuntimeCapabilities,
  downloadWhisperModel,
  inspectMedia,
  installLocalFfmpegRuntime,
  isTauriRuntime,
  listenToModelDownloadProgress,
  listWhisperModels,
  listenToPipelineProgress,
  listenToRuntimeDownloadProgress,
  loadAppSettings,
  loadProjectSnapshot,
  cleanupJobCache,
  pathExists,
  persistAppSettings,
  removeLocalFfmpegRuntime,
  removeWhisperModel,
  renderHardSubtitle,
  saveProjectSnapshot,
  transcribeDirectWithGemini,
  transcribeWithWhisper,
  translateSegmentsWithGemini
} from '$lib/services/tauri';
import { segmentsToSrt, validateSegments } from '$lib/srt';
import type {
  AppSettings,
  JobFormState,
  JobPhase,
  JobRecord,
  MediaInspection,
  ModelDownloadProgressEvent,
  PipelineProgressEvent,
  ProcessLogEntry,
  ProjectSnapshot,
  RuntimeDownloadProgressEvent,
  RuntimeCapabilities,
  SubtitleSegment,
  ViewKey,
  WhisperModelOption
} from '$lib/types';
import { createId } from '$lib/utils';
import { get, writable } from 'svelte/store';

interface PersistedAppState {
  history: JobRecord[];
}

function loadPersistedState(): PersistedAppState {
  if (typeof window === 'undefined') {
    return {
      history: []
    };
  }

  const raw = window.localStorage.getItem(APP_STORAGE_KEY);
  if (!raw) {
    return {
      history: []
    };
  }

  try {
    const parsed = JSON.parse(raw) as Partial<PersistedAppState>;
    return {
      history: parsed.history ?? []
    };
  } catch {
    return {
      history: []
    };
  }
}

function persistState(historyData: JobRecord[]): void {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    window.localStorage.setItem(
      APP_STORAGE_KEY,
      JSON.stringify({
        history: historyData
      } satisfies PersistedAppState)
    );
  } catch {
    // localStorage quota exceeded — silently ignore to avoid crashing the app.
  }
}

function createDraftJob(form: JobFormState, settings?: AppSettings): JobRecord {
  const timestamp = new Date().toISOString();

  // Use saved translation instruction if available
  const translationInstruction = settings?.savedTranslationInstruction || form.translationInstruction;

  return {
    id: createId('job'),
    name: form.inputPath ? form.inputPath.split(/[\\/]/).pop() ?? 'Untitled media' : 'Untitled media',
    createdAt: timestamp,
    updatedAt: timestamp,
    status: 'draft',
    phase: 'idle',
    progress: 0,
    etaSeconds: null,
    message: 'Sẵn sàng để chạy pipeline.',
    inspection: null,
    sourceSegments: [],
    translatedSegments: [],
    form: {
      ...form,
      translationInstruction
    },
    style: {
      ...DEFAULT_STYLE
    },
    outputs: {}
  };
}

function updateJob(job: JobRecord, patch: Partial<JobRecord>): JobRecord {
  return {
    ...job,
    ...patch,
    updatedAt: new Date().toISOString()
  };
}

function upsertHistoryEntry(history: JobRecord[], next: JobRecord): JobRecord[] {
  const currentIndex = history.findIndex((entry) => entry.id === next.id);
  if (currentIndex === -1) {
    // Prepend new entry and cap at MAX_HISTORY_ENTRIES.
    return [next, ...history].slice(0, MAX_HISTORY_ENTRIES);
  }

  const updated = [...history];
  updated[currentIndex] = next;
  return updated;
}

const persisted = loadPersistedState();
let pipelineUnlisten: (() => void) | null = null;
let modelDownloadUnlisten: (() => void) | null = null;
let runtimeDownloadUnlisten: (() => void) | null = null;

export const currentView = writable<ViewKey>('translate');
export const runtimeCapabilities = writable<RuntimeCapabilities | null>(null);
export const whisperModels = writable<WhisperModelOption[]>(WHISPER_MODELS);
export const activeJob = writable<JobRecord>(createDraftJob(DEFAULT_FORM, DEFAULT_SETTINGS));
export const history = writable<JobRecord[]>(persisted.history);
export const settings = writable<AppSettings>(DEFAULT_SETTINGS);
export const inspection = writable<MediaInspection | null>(null);
export const busy = writable(false);
export const inspectingMedia = writable(false);
export const installingRuntime = writable(false);
export const removingRuntime = writable(false);
export const cancelRequested = writable(false);
export const statusMessage = writable('Đang khởi tạo workspace.');
export const validationErrors = writable<string[]>([]);
export const downloadingModelId = writable<string | null>(null);
export const downloadingModelProgress = writable<number>(0);
export const deletingModelId = writable<string | null>(null);
export const runtimeDownloadProgress = writable<number>(0);
export const runtimeDownloadMessage = writable('');
export const processLogs = writable<ProcessLogEntry[]>([]);

let persistTimer: ReturnType<typeof setTimeout> | null = null;
let subtitleUpdateTimer: ReturnType<typeof setTimeout> | null = null;

// Undo/Redo state
/**
 * Undo/Redo uses a patch-based approach to avoid storing full segment array copies.
 * Each undo entry records only which segment changed and what its previous value was.
 */
interface UndoPatch {
  segmentId: string;
  translated: boolean;
  previous: SubtitleSegment;
}

interface UndoFullSnapshot {
  type: 'full';
  translatedSegments: SubtitleSegment[];
  sourceSegments: SubtitleSegment[];
}

type UndoEntry = UndoPatch | UndoFullSnapshot;

function isFullSnapshot(entry: UndoEntry): entry is UndoFullSnapshot {
  return 'type' in entry && entry.type === 'full';
}

const MAX_UNDO_HISTORY = 50;
let undoStack: UndoEntry[] = [];
let redoStack: UndoEntry[] = [];
let isUndoRedoAction = false;

export const canUndo = writable(false);
export const canRedo = writable(false);

function updateUndoRedoState(): void {
  canUndo.set(undoStack.length > 0);
  canRedo.set(redoStack.length > 0);
}

function pushUndoPatch(segmentId: string, translated: boolean, previous: SubtitleSegment): void {
  if (isUndoRedoAction) return;

  undoStack.push({ segmentId, translated, previous: { ...previous } });
  if (undoStack.length > MAX_UNDO_HISTORY) {
    undoStack.shift();
  }
  redoStack = [];
  updateUndoRedoState();
}

/** Push a full snapshot for bulk operations (e.g. import, batch translate). */
export function pushUndoSnapshot(): void {
  if (isUndoRedoAction) return;

  const current = get(activeJob);
  undoStack.push({
    type: 'full',
    translatedSegments: current.translatedSegments.map(s => ({ ...s })),
    sourceSegments: current.sourceSegments.map(s => ({ ...s }))
  });
  if (undoStack.length > MAX_UNDO_HISTORY) {
    undoStack.shift();
  }
  redoStack = [];
  updateUndoRedoState();
}

function applyPatchUndo(entry: UndoPatch, current: JobRecord): { next: JobRecord; reverse: UndoEntry } {
  const key = entry.translated ? 'translatedSegments' : 'sourceSegments';
  const segments = current[key];
  const idx = segments.findIndex(s => s.id === entry.segmentId);

  // Build reverse patch from current state before applying
  const currentSegment = idx !== -1 ? { ...segments[idx] } : entry.previous;
  const reverse: UndoPatch = {
    segmentId: entry.segmentId,
    translated: entry.translated,
    previous: currentSegment
  };

  if (idx !== -1) {
    const newSegments = [...segments];
    newSegments[idx] = { ...entry.previous };
    return { next: updateJob(current, { [key]: newSegments }), reverse };
  }

  return { next: current, reverse };
}

function applySnapshotUndo(entry: UndoFullSnapshot, current: JobRecord): { next: JobRecord; reverse: UndoEntry } {
  const reverse: UndoFullSnapshot = {
    type: 'full',
    translatedSegments: current.translatedSegments.map(s => ({ ...s })),
    sourceSegments: current.sourceSegments.map(s => ({ ...s }))
  };

  return {
    next: updateJob(current, {
      translatedSegments: entry.translatedSegments,
      sourceSegments: entry.sourceSegments
    }),
    reverse
  };
}

export function undo(): void {
  if (undoStack.length === 0) return;

  const current = get(activeJob);
  const entry = undoStack.pop()!;

  isUndoRedoAction = true;
  const { next, reverse } = isFullSnapshot(entry)
    ? applySnapshotUndo(entry, current)
    : applyPatchUndo(entry, current);

  redoStack.push(reverse);
  replaceActiveJob(next);
  isUndoRedoAction = false;

  updateUndoRedoState();
  validationErrors.set(validateSegments(next.translatedSegments));
}

export function redo(): void {
  if (redoStack.length === 0) return;

  const current = get(activeJob);
  const entry = redoStack.pop()!;

  isUndoRedoAction = true;
  const { next, reverse } = isFullSnapshot(entry)
    ? applySnapshotUndo(entry, current)
    : applyPatchUndo(entry, current);

  undoStack.push(reverse);
  replaceActiveJob(next);
  isUndoRedoAction = false;

  updateUndoRedoState();
  validationErrors.set(validateSegments(next.translatedSegments));
}

export function clearUndoRedo(): void {
  undoStack = [];
  redoStack = [];
  updateUndoRedoState();
}

function schedulePersist(): void {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    const $history = get(history);
    persistState($history);
  }, PERSIST_DEBOUNCE_MS);
}

history.subscribe(() => {
  schedulePersist();
});

// Settings are persisted to the Tauri file system (single source of truth),
// not to localStorage. See saveApiKey() and saveTranslationInstruction().

function replaceActiveJob(next: JobRecord): void {
  activeJob.set(next);
  inspection.set(next.inspection ?? null);
}

export function recordProcessError(context: string, error: unknown, fallback: string): string {
  const message = getErrorMessage(error, fallback);
  const details = getErrorDetails(error);
  const job = get(activeJob);

  processLogs.update((items) =>
    [
      {
        id: createId('log'),
        createdAt: new Date().toISOString(),
        context,
        message,
        detail: details,
        stack: null,
        jobId: job.id,
        jobName: job.name,
        jobPhase: job.phase
      },
      ...items
    ].slice(0, MAX_LOG_ENTRIES)
  );

  return message;
}

function errorDetail(error: unknown): { detail: string | null; stack: string | null } {
  if (error instanceof Error) {
    const detailParts = [error.name !== 'Error' ? error.name : null];

    if (typeof error.cause === 'string') {
      detailParts.push(error.cause);
    } else if (error.cause) {
      try {
        detailParts.push(JSON.stringify(error.cause, null, 2));
      } catch {
        detailParts.push(String(error.cause));
      }
    }

    return {
      detail: detailParts.filter(Boolean).join('\n') || null,
      stack: error.stack ?? null
    };
  }

  if (typeof error === 'string') {
    return {
      detail: error,
      stack: null
    };
  }

  if (!error) {
    return {
      detail: null,
      stack: null
    };
  }

  try {
    return {
      detail: JSON.stringify(error, null, 2),
      stack: null
    };
  } catch {
    return {
      detail: String(error),
      stack: null
    };
  }
}

function applyPipelineEvent(event: PipelineProgressEvent): void {
  const current = get(activeJob);
  if (current.id !== event.jobId) {
    return;
  }

  const phase = event.phase as JobPhase;
  const next = updateJob(current, {
    phase,
    progress: event.progress,
    etaSeconds: event.etaSeconds ?? null,
    message: event.message,
    status:
      phase === 'failed'
        ? 'failed'
        : phase === 'cancelled'
          ? 'cancelled'
          : phase === 'complete' || phase === 'ready'
            ? 'completed'
            : 'running'
  });

  replaceActiveJob(next);
  
  // Only save to history on success (completed status)
  if (next.status === 'completed') {
    history.update((items) => upsertHistoryEntry(items, next));
  }
  
  statusMessage.set(event.message);
}

// Throttle timer cho model download progress
let _downloadProgressTimer: ReturnType<typeof setTimeout> | null = null;
let _pendingDownloadProgress: ModelDownloadProgressEvent | null = null;

function clearDownloadProgressTimer(): void {
  if (_downloadProgressTimer) {
    clearTimeout(_downloadProgressTimer);
    _downloadProgressTimer = null;
  }
  _pendingDownloadProgress = null;
}

function applyModelDownloadEvent(event: ModelDownloadProgressEvent): void {
  if (get(downloadingModelId) !== event.modelId) {
    return;
  }

  // Store latest event
  _pendingDownloadProgress = event;

  // Throttle: chỉ update UI tối đa mỗi 200ms để tránh nhấp nháy
  if (_downloadProgressTimer) {
    return;
  }

  _downloadProgressTimer = setTimeout(() => {
    _downloadProgressTimer = null;
    if (_pendingDownloadProgress) {
      const pending = _pendingDownloadProgress;
      _pendingDownloadProgress = null;
      
      // Skip nếu progress không đổi
      const currentProgress = get(downloadingModelProgress);
      if (pending.progress !== currentProgress) {
        downloadingModelProgress.set(pending.progress);
        statusMessage.set(`Đang tải Whisper model ${pending.modelId} (${pending.progress}%).`);
      }
    }
  }, MODEL_DOWNLOAD_PROGRESS_THROTTLE_MS);
}

function applyRuntimeDownloadEvent(event: RuntimeDownloadProgressEvent): void {
  if (get(installingRuntime)) {
    runtimeDownloadProgress.set(event.progress);
    runtimeDownloadMessage.set(event.message);
    statusMessage.set(event.message);
  }
}

export async function bootstrapApp(): Promise<void> {
  statusMessage.set('Đang kiểm tra runtime và model local.');

  try {
    const [capabilities, models] = await Promise.all([
      detectRuntimeCapabilities(),
      listWhisperModels().catch(() => WHISPER_MODELS)
    ]);

    runtimeCapabilities.set(capabilities);
    whisperModels.set(models.length > 0 ? models : WHISPER_MODELS);

    // Load settings from Tauri backend (single source of truth).
    if (isTauriRuntime()) {
      try {
        const loadedSettings = await loadAppSettings();
        settings.update(current => ({ ...current, ...loadedSettings }));

        // Restore saved translation instruction into the active job draft.
        if (loadedSettings.savedTranslationInstruction) {
          const current = get(activeJob);
          replaceActiveJob(
            updateJob(current, {
              form: {
                ...current.form,
                translationInstruction: loadedSettings.savedTranslationInstruction
              }
            })
          );
        }
      } catch (error) {
        recordProcessError('Restore settings', error, 'Không thể khôi phục settings đã lưu.');
      }
    }

    statusMessage.set('Workspace đã sẵn sàng.');
  } catch (error) {
    recordProcessError('Workspace bootstrap', error, 'Không thể khởi tạo workspace.');
  }

  // Set up event listeners independently so one failure doesn't block others.
  if (!pipelineUnlisten) {
    try {
      pipelineUnlisten = await listenToPipelineProgress(applyPipelineEvent);
    } catch (error) {
      recordProcessError('Listen pipeline progress', error, 'Không thể đăng ký event pipeline.');
    }
  }

  if (!modelDownloadUnlisten) {
    try {
      modelDownloadUnlisten = await listenToModelDownloadProgress(applyModelDownloadEvent);
    } catch (error) {
      recordProcessError('Listen model download', error, 'Không thể đăng ký event tải model.');
    }
  }

  if (!runtimeDownloadUnlisten) {
    try {
      runtimeDownloadUnlisten = await listenToRuntimeDownloadProgress(applyRuntimeDownloadEvent);
    } catch (error) {
      recordProcessError('Listen runtime download', error, 'Không thể đăng ký event tải runtime.');
    }
  }
}

/**
 * Cleanup function to properly unregister all event listeners.
 * Should be called when the app is unmounting or shutting down.
 */
export function cleanupEventListeners(): void {
  if (pipelineUnlisten) {
    pipelineUnlisten();
    pipelineUnlisten = null;
  }

  if (modelDownloadUnlisten) {
    modelDownloadUnlisten();
    modelDownloadUnlisten = null;
  }

  if (runtimeDownloadUnlisten) {
    runtimeDownloadUnlisten();
    runtimeDownloadUnlisten = null;
  }

  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }

  if (subtitleUpdateTimer) {
    clearTimeout(subtitleUpdateTimer);
    subtitleUpdateTimer = null;
  }
}

export async function installManagedFfmpeg(): Promise<void> {
  installingRuntime.set(true);
  runtimeDownloadProgress.set(0);
  runtimeDownloadMessage.set('Đang chuẩn bị tải FFmpeg local.');
  statusMessage.set('Đang cài FFmpeg local vào app.');

  try {
    const capabilities = await installLocalFfmpegRuntime();
    runtimeCapabilities.set(capabilities);
    runtimeDownloadProgress.set(PROGRESS_COMPLETE);
    runtimeDownloadMessage.set('FFmpeg local đã sẵn sàng.');
    statusMessage.set('FFmpeg local đã sẵn sàng.');
  } catch (error) {
    const message = recordProcessError('Install FFmpeg', error, 'Cài FFmpeg local thất bại.');
    runtimeDownloadMessage.set(message);
  } finally {
    installingRuntime.set(false);
  }
}

export async function removeManagedFfmpeg(): Promise<void> {
  removingRuntime.set(true);
  statusMessage.set('Đang xóa FFmpeg local khỏi app.');

  try {
    const capabilities = await removeLocalFfmpegRuntime();
    runtimeCapabilities.set(capabilities);
    runtimeDownloadProgress.set(0);
    runtimeDownloadMessage.set('');
    statusMessage.set('Đã xóa FFmpeg local.');
  } catch (error) {
    recordProcessError('Remove FFmpeg', error, 'Xóa FFmpeg local thất bại.');
  } finally {
    removingRuntime.set(false);
  }
}

export async function removeManagedWhisper(): Promise<void> {
  // Whisper is now bundled via whisper-rs; no separate runtime removal needed.
  // This function is kept for backward compatibility with the UI.
}

export function resetActiveJob(): void {
  cancelRequested.set(false);
  validationErrors.set([]);
  replaceActiveJob(createDraftJob(DEFAULT_FORM, get(settings)));
}

export function setView(view: ViewKey): void {
  currentView.set(view);
}

export function updateForm<K extends keyof JobFormState>(key: K, value: JobFormState[K]): void {
  const next = updateJob(get(activeJob), {
    form: {
      ...get(activeJob).form,
      [key]: value
    }
  });
  replaceActiveJob(next);
  
  // Only save to history if job is completed
  if (next.status === 'completed') {
    history.update((items) => upsertHistoryEntry(items, next));
  }

  // Note: translationInstruction is now saved manually via saveTranslationInstruction()
}

export async function saveTranslationInstruction(instruction: string): Promise<void> {
  const nextSettings = {
    ...get(settings),
    savedTranslationInstruction: instruction
  };
  settings.set(nextSettings);
  if (isTauriRuntime()) {
    await persistAppSettings(nextSettings);
  }
}

export function updateStyle(
  key: keyof JobRecord['style'],
  value: JobRecord['style'][keyof JobRecord['style']]
): void {
  const current = get(activeJob);
  const next = updateJob(current, {
    style: {
      ...current.style,
      [key]: value
    }
  });
  replaceActiveJob(next);
  
  // Only save to history if job is completed
  if (next.status === 'completed') {
    history.update((items) => upsertHistoryEntry(items, next));
  }
}

export function updateSubtitleSegment(
  segmentId: string,
  patch: Partial<SubtitleSegment>,
  translated = true
): void {
  const current = get(activeJob);
  const key = translated ? 'translatedSegments' : 'sourceSegments';
  const segments = current[key];
  const idx = segments.findIndex((s) => s.id === segmentId);

  // Save single-segment patch for undo before modification
  if (!isUndoRedoAction && idx !== -1) {
    pushUndoPatch(segmentId, translated, segments[idx]);
  }

  if (idx !== -1) {
    // Create new array with updated segment (no in-place mutation)
    const newSegments = [...segments];
    newSegments[idx] = { ...segments[idx], ...patch };

    const next = updateJob(current, { [key]: newSegments });
    replaceActiveJob(next);

    // Debounce validation to avoid expensive validation on every keystroke
    if (subtitleUpdateTimer) {
      clearTimeout(subtitleUpdateTimer);
    }
    subtitleUpdateTimer = setTimeout(() => {
      subtitleUpdateTimer = null;
      validationErrors.set(validateSegments(translated ? next.translatedSegments : next.sourceSegments));
    }, SUBTITLE_UPDATE_DEBOUNCE_MS);
  }
}

export async function saveApiKey(apiKey: string): Promise<void> {
  const nextSettings = {
    ...get(settings),
    apiKey
  };

  settings.set(nextSettings);
  if (isTauriRuntime()) {
    await persistAppSettings(nextSettings);
  }
}

/**
 * Returns the current API key from the settings store.
 * Settings are already loaded from the Tauri backend during bootstrapApp().
 */
export function restoreSavedApiKey(): string {
  return get(settings).apiKey;
}

export async function inspectSelectedMedia(filePath: string): Promise<void> {
  busy.set(true);
  inspectingMedia.set(true);
  statusMessage.set('Đang đọc metadata media.');

  try {
    const data = await inspectMedia(filePath);
    inspection.set(data);

    // Validate audio tracks
    if (!data.audioTracks || data.audioTracks.length === 0) {
      throw new Error('Media không có audio track nào. Vui lòng chọn file có audio.');
    }

    const current = get(activeJob);
    const next = updateJob(current, {
      name: filePath.split(/[\\/]/).pop() ?? current.name,
      inspection: data,
      form: {
        ...current.form,
        inputPath: filePath,
        inputKind: data.kind,
        selectedAudioTrack: data.audioTracks[0]?.index ?? 0
      }
    });

    replaceActiveJob(next);
    statusMessage.set('Đã đọc xong metadata media.');
  } catch (error) {
    recordProcessError('Inspect media', error, 'Không thể đọc metadata media.');
  } finally {
    busy.set(false);
    inspectingMedia.set(false);
  }
}

export async function downloadModel(modelId: string): Promise<void> {
  downloadingModelId.set(modelId);
  downloadingModelProgress.set(0);
  statusMessage.set(`Đang tải Whisper model ${modelId}.`);

  try {
    const model = await downloadWhisperModel(modelId);
    whisperModels.update((items) => items.map((item) => (item.id === model.id ? model : item)));
    downloadingModelProgress.set(PROGRESS_COMPLETE);
    statusMessage.set(`Whisper model ${modelId} đã sẵn sàng.`);
  } catch (error) {
    recordProcessError('Download Whisper model', error, `Tải Whisper model ${modelId} thất bại.`);
  } finally {
    clearDownloadProgressTimer();
    downloadingModelId.set(null);
    downloadingModelProgress.set(0);
  }
}

export function cancelModelDownload(): void {
  const modelId = get(downloadingModelId);
  if (modelId) {
    clearDownloadProgressTimer();
    downloadingModelId.set(null);
    downloadingModelProgress.set(0);
    statusMessage.set(`Đã hủy tải model ${modelId}.`);
  }
}

export async function deleteModel(modelId: string): Promise<void> {
  deletingModelId.set(modelId);
  statusMessage.set(`Đang xóa Whisper model ${modelId}.`);

  try {
    const model = await removeWhisperModel(modelId);
    whisperModels.update((items) => items.map((item) => (item.id === model.id ? model : item)));
    statusMessage.set(`Đã xóa Whisper model ${modelId}.`);
  } catch (error) {
    recordProcessError('Remove Whisper model', error, `Xóa Whisper model ${modelId} thất bại.`);
  } finally {
    deletingModelId.set(null);
  }
}

export async function requestCancellation(): Promise<void> {
  cancelRequested.set(true);
  const jobId = get(activeJob).id;
  const next = withPhase(get(activeJob), 'cancelled', get(activeJob).progress, 'Đã yêu cầu hủy job.');
  replaceActiveJob(next);
  // Don't save cancelled jobs to history
  await cancelPipelineProcess(jobId);
}

function throwIfCancelled(): void {
  if (get(cancelRequested)) {
    throw new Error('Job đã bị hủy bởi người dùng.');
  }
}

function withPhase(job: JobRecord, phase: JobPhase, progress: number, message: string): JobRecord {
  return updateJob(job, {
    phase,
    progress,
    message,
    status:
      phase === 'failed'
        ? 'failed'
        : phase === 'complete' || phase === 'ready'
          ? 'completed'
          : phase === 'cancelled'
            ? 'cancelled'
            : 'running'
  });
}

export async function startPipeline(apiKey: string): Promise<void> {
  validationErrors.set([]);
  busy.set(true);
  cancelRequested.set(false);

  let job = updateJob(get(activeJob), {
    status: 'running',
    phase: 'inspect',
    progress: 5,
    message: 'Đang chuẩn bị pipeline.'
  });

  replaceActiveJob(job);
  // Don't save running jobs to history

  try {
    if (!job.form.inputPath) {
      throw new Error('Bạn cần chọn file video/audio trước khi chạy.');
    }

    const caps = get(runtimeCapabilities);
    const ffmpegReady = caps?.ffmpegAvailable || caps?.localFfmpegInstalled;
    if (!ffmpegReady) {
      throw new Error(
        'FFmpeg chưa được cài đặt. Vào tab "Cài đặt" để cài FFmpeg trước khi chạy pipeline.'
      );
    }

    throwIfCancelled();

    if (!job.inspection) {
      const media = await inspectMedia(job.form.inputPath);
      job = updateJob(job, { inspection: media });
    }

    throwIfCancelled();

    const sourcePath = job.inspection!.path;
    const trackIndex = job.form.selectedAudioTrack;
    const durationSeconds = job.inspection?.durationSeconds ?? 1;

    let sourceSegments: SubtitleSegment[] = [];
    let translatedSegments: SubtitleSegment[] = [];

    if (job.form.processingMode === 'whisper_translate') {
      // whisper-rs is bundled; only a downloaded model file is needed.
      const modelReady = job.form.whisperModelId;
      if (!modelReady) {
        throw new Error('Chưa chọn Whisper model. Hãy chọn model trước khi dùng chế độ này.');
      }

      job = withPhase(job, 'transcribe', 10, 'Whisper đang tạo transcript có timestamp.');
      replaceActiveJob(job);

      sourceSegments = await transcribeWithWhisper({
        jobId: job.id,
        audioPath: sourcePath,
        trackIndex,
        sourceLanguage: job.form.sourceLanguage,
        modelId: job.form.whisperModelId,
        cpuOnly: job.form.cpuOnly
      });

      throwIfCancelled();

      // Immediately show empty translated segments (don't copy source text)
      const interimTranslatedSegments = sourceSegments.map(seg => ({
        ...seg,
        translatedText: ''
      }));

      job = updateJob(job, {
        sourceSegments,
        translatedSegments: interimTranslatedSegments
      });
      replaceActiveJob(job);

      job = withPhase(job, 'translate', 70, 'Gemini đang dịch subtitle sang tiếng Việt.');
      replaceActiveJob(job);

      // Wrap Gemini translation in try/catch for partial success handling
      try {
        const result = await translateSegmentsWithGemini({
          jobId: job.id,
          apiKey,
          modelId: job.form.geminiModelId,
          translationInstruction: job.form.translationInstruction,
          segments: sourceSegments
        });
        // SRT dịch là phần riêng biệt, không cần merge với SRT gốc
        translatedSegments = result.segments;
        
        // Save token usage to job
        job = updateJob(job, { tokenUsage: result.tokenUsage });
      } catch (geminiError) {
        // Gemini failed, but source subtitles are preserved
        const errorMessage = geminiError instanceof Error ? geminiError.message : String(geminiError);
        job = updateJob(
          withPhase(job, 'ready', PROGRESS_COMPLETE, `Dịch thất bại: ${errorMessage}. Đã giữ lại subtitle ngôn ngữ gốc.`),
          {
            status: 'partial',
            sourceSegments,
            translatedSegments: interimTranslatedSegments
          }
        );
        replaceActiveJob(job);
        history.update((items) => upsertHistoryEntry(items, job));
        // Cleanup cache on partial completion.
        cleanupJobCache(job.id).catch(() => {});
        return; // Exit early - job is saved with source-language subs
      }
    } else {
      job = withPhase(
        job,
        'transcribe',
        10,
        'Gemini direct audio đang xử lý transcript và translation theo chế độ experimental.'
      );
      replaceActiveJob(job);

      const result = await transcribeDirectWithGemini({
        jobId: job.id,
        apiKey,
        modelId: job.form.geminiModelId,
        audioPath: sourcePath,
        trackIndex,
        durationSeconds,
        sourceLanguage: job.form.sourceLanguage,
        translationInstruction: job.form.translationInstruction
      });
      throwIfCancelled();
      sourceSegments = result.segments;
      translatedSegments = [...sourceSegments];
      
      // Save token usage to job
      job = updateJob(job, { tokenUsage: result.tokenUsage });
    }

    job = updateJob(
      withPhase(job, 'ready', PROGRESS_COMPLETE, 'Subtitle đã sẵn sàng để chỉnh sửa và export.'),
      {
        status: 'completed',
        sourceSegments,
        translatedSegments
      }
    );

    replaceActiveJob(job);
    // Save to history only when completed successfully
    history.update((items) => upsertHistoryEntry(items, job));
    // Cleanup cache after successful completion.
    cleanupJobCache(job.id).catch(() => {});
  } catch (error) {
    const isCancelled = error instanceof Error && error.message === 'Job đã bị hủy bởi người dùng.';
    const message = getErrorMessage(error, 'Pipeline thất bại.');
    const next = updateJob(
      withPhase(
        job,
        isCancelled ? 'cancelled' : 'failed',
        job.progress,
        message
      ),
      {
        status: isCancelled ? 'cancelled' : 'failed'
      }
    );
    replaceActiveJob(next);
    // Don't save failed or cancelled jobs to history
    if (!isCancelled) {
      recordProcessError('Pipeline', error, 'Pipeline thất bại.');
    }
  } finally {
    cancelRequested.set(false);
    busy.set(false);
  }
}

/**
 * Re-run only the Gemini translation step using existing sourceSegments.
 * Skips Whisper transcription entirely.
 */
export async function retranslateOnly(apiKey: string): Promise<void> {
  validationErrors.set([]);
  busy.set(true);
  cancelRequested.set(false);

  let job = get(activeJob);

  if (job.sourceSegments.length === 0) {
    busy.set(false);
    throw new Error('Chưa có SRT gốc (source segments). Cần chạy pipeline đầy đủ trước.');
  }

  job = updateJob(job, {
    status: 'running',
    phase: 'translate',
    progress: 70,
    message: 'Gemini đang dịch lại subtitle sang tiếng Việt.'
  });
  replaceActiveJob(job);

  try {
    throwIfCancelled();

    const result = await translateSegmentsWithGemini({
      jobId: job.id,
      apiKey,
      modelId: job.form.geminiModelId,
      translationInstruction: job.form.translationInstruction,
      segments: job.sourceSegments
    });

    throwIfCancelled();

    job = updateJob(
      withPhase(job, 'ready', PROGRESS_COMPLETE, 'Subtitle đã sẵn sàng để chỉnh sửa và export.'),
      {
        status: 'completed',
        translatedSegments: result.segments,
        tokenUsage: result.tokenUsage
      }
    );
    replaceActiveJob(job);
    history.update((items) => upsertHistoryEntry(items, job));
    cleanupJobCache(job.id).catch(() => {});
  } catch (error) {
    const isCancelled = error instanceof Error && error.message === 'Job đã bị hủy bởi người dùng.';
    const message = getErrorMessage(error, 'Dịch lại thất bại.');

    if (isCancelled) {
      const next = updateJob(
        withPhase(job, 'cancelled', job.progress, message),
        { status: 'cancelled' }
      );
      replaceActiveJob(next);
    } else {
      // Keep existing translatedSegments (even if source-language) on failure
      const next = updateJob(
        withPhase(job, 'ready', PROGRESS_COMPLETE, `Dịch lại thất bại: ${message}`),
        { status: 'partial' }
      );
      replaceActiveJob(next);
      history.update((items) => upsertHistoryEntry(items, next));
      recordProcessError('Retranslate', error, 'Dịch lại thất bại.');
    }
  } finally {
    cancelRequested.set(false);
    busy.set(false);
  }
}

export async function exportProject(path: string): Promise<void> {
  const snapshot: ProjectSnapshot = {
    version: PROJECT_SCHEMA_VERSION,
    exportedAt: new Date().toISOString(),
    job: get(activeJob)
  };

  await saveProjectSnapshot({
    path,
    snapshot
  });

  settings.update((current) => ({
    ...current,
    lastOpenedProjectPath: path
  }));
}

export async function importProject(path: string): Promise<void> {
  const snapshot = await loadProjectSnapshot({ path });
  pushUndoSnapshot(); // Save current state before replacing with imported project
  replaceActiveJob(snapshot.job);
  // Only save to history if imported job is completed
  if (snapshot.job.status === 'completed') {
    history.update((items) => upsertHistoryEntry(items, snapshot.job));
  }
  settings.update((current) => ({
    ...current,
    lastOpenedProjectPath: path
  }));
}

export function relinkProjectMedia(path: string): void {
  const current = get(activeJob);
  const next = updateJob(current, {
    name: path.split(/[\\/]/).pop() ?? current.name,
    form: {
      ...current.form,
      inputPath: path
    }
  });

  replaceActiveJob(next);
}

export function loadJobFromHistory(job: JobRecord): void {
  replaceActiveJob(job);
}

export function deleteHistoryItem(jobId: string): void {
  const current = get(activeJob);
  
  // If the deleted item is the currently active job, reset to a fresh draft
  if (current.id === jobId) {
    resetActiveJob();
  }
  
  history.update((items) => items.filter((item) => item.id !== jobId));

  // Cleanup cached files for this job (fire-and-forget).
  cleanupJobCache(jobId).catch(() => {});
}

export function clearHistory(): void {
  // Cleanup cached files for all jobs before clearing.
  const items = get(history);
  for (const item of items) {
    cleanupJobCache(item.id).catch(() => {});
  }

  // Reset active job to fresh draft
  resetActiveJob();
  
  // Clear all history
  history.set([]);
}

export async function renderCurrentVideo(outputPath: string): Promise<void> {
  const job = get(activeJob);
  if (!job.form.inputPath || job.translatedSegments.length === 0 || !job.inspection) {
    recordProcessError('Render video', new Error('Chưa có đủ dữ liệu để render video.'), 'Chưa có đủ dữ liệu để render video.');
    return;
  }

  const caps = get(runtimeCapabilities);
  const ffmpegReady = caps?.ffmpegAvailable || caps?.localFfmpegInstalled;
  if (!ffmpegReady) {
    recordProcessError(
      'Render video',
      new Error('FFmpeg chưa được cài đặt. Vào tab "Cài đặt" để cài FFmpeg trước khi render video.'),
      'FFmpeg chưa được cài đặt.'
    );
    return;
  }

  busy.set(true);
  try {
    const rendering = withPhase(job, 'render', 72, 'Đang render hard subtitle qua FFmpeg sidecar.');
    replaceActiveJob(rendering);
    // Don't save rendering jobs to history

    const renderedVideoPath = await renderHardSubtitle({
      jobId: rendering.id,
      inputPath: rendering.form.inputPath,
      outputPath,
      subtitleContent: segmentsToSrt(rendering.translatedSegments, true),
      style: rendering.style,
      durationSeconds: rendering.inspection?.durationSeconds ?? 1
    });

    const next = updateJob(get(activeJob), {
      phase: 'ready',
      status: 'completed',
      progress: 100,
      message: 'Render video hoàn tất.',
      outputs: {
        ...get(activeJob).outputs,
        renderedVideoPath
      }
    });
    replaceActiveJob(next);
    history.update((items) => upsertHistoryEntry(items, next));
  } catch (error) {
    const failed = withPhase(get(activeJob), 'failed', get(activeJob).progress, getErrorMessage(error, 'Render video thất bại.'));
    replaceActiveJob(failed);
    // Don't save failed jobs to history
    recordProcessError('Render video', error, 'Render video thất bại.');
  } finally {
    busy.set(false);
  }
}
