import {
  APP_STORAGE_KEY,
  DEFAULT_FORM,
  DEFAULT_SETTINGS,
  DEFAULT_STYLE,
  PROJECT_SCHEMA_VERSION,
  WHISPER_MODELS
} from '$lib/constants';
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
  normalizeAudio,
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
  settings: AppSettings;
}

function loadPersistedState(): PersistedAppState {
  if (typeof window === 'undefined') {
    return {
      history: [],
      settings: DEFAULT_SETTINGS
    };
  }

  const raw = window.localStorage.getItem(APP_STORAGE_KEY);
  if (!raw) {
    return {
      history: [],
      settings: DEFAULT_SETTINGS
    };
  }

  try {
    const parsed = JSON.parse(raw) as Partial<PersistedAppState>;
    return {
      history: parsed.history ?? [],
      settings: {
        ...DEFAULT_SETTINGS,
        ...(parsed.settings ?? {})
      }
    };
  } catch {
    return {
      history: [],
      settings: DEFAULT_SETTINGS
    };
  }
}

function persistState(history: JobRecord[], settings: AppSettings): void {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(
    APP_STORAGE_KEY,
    JSON.stringify({
      history,
      settings
    } satisfies PersistedAppState)
  );
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
    normalizedAudioPath: null,
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
    return [next, ...history];
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
export const activeJob = writable<JobRecord>(createDraftJob(DEFAULT_FORM, persisted.settings));
export const history = writable<JobRecord[]>(persisted.history);
export const settings = writable<AppSettings>(persisted.settings);
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

function schedulePersist(): void {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistState(get(history), get(settings));
    persistTimer = null;
  }, 100);
}

history.subscribe(() => {
  schedulePersist();
});

settings.subscribe(() => {
  schedulePersist();
});

function replaceActiveJob(next: JobRecord): void {
  activeJob.set(next);
  inspection.set(next.inspection ?? null);
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
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

export function recordProcessError(context: string, error: unknown, fallback: string): string {
  const message = errorMessage(error, fallback);
  const { detail, stack } = errorDetail(error);
  const job = get(activeJob);

  processLogs.update((items) =>
    [
      {
        id: createId('log'),
        createdAt: new Date().toISOString(),
        context,
        message,
        detail,
        stack,
        jobId: job.id,
        jobName: job.name,
        phase: job.phase,
        progress: job.progress
      },
      ...items
    ].slice(0, 200)
  );

  statusMessage.set(message);
  return message;
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

function applyModelDownloadEvent(event: ModelDownloadProgressEvent): void {
  if (get(downloadingModelId) !== event.modelId) {
    return;
  }

  downloadingModelProgress.set(event.progress);
  statusMessage.set(`Đang tải Whisper model ${event.modelId} (${event.progress}%).`);
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

    if (!pipelineUnlisten) {
      pipelineUnlisten = await listenToPipelineProgress(applyPipelineEvent);
    }

    if (!modelDownloadUnlisten) {
      modelDownloadUnlisten = await listenToModelDownloadProgress(applyModelDownloadEvent);
    }

    if (!runtimeDownloadUnlisten) {
      runtimeDownloadUnlisten = await listenToRuntimeDownloadProgress(applyRuntimeDownloadEvent);
    }

    statusMessage.set('Workspace đã sẵn sàng.');
  } catch (error) {
    recordProcessError('Workspace bootstrap', error, 'Không thể khởi tạo workspace.');
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
    runtimeDownloadProgress.set(100);
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
  const segments = current[key].map((segment) =>
    segment.id === segmentId ? { ...segment, ...patch } : segment
  );

  const next = updateJob(current, {
    [key]: segments
  });

  replaceActiveJob(next);
  validationErrors.set(validateSegments(translated ? next.translatedSegments : next.sourceSegments));
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

export async function restoreSavedApiKey(): Promise<string> {
  if (isTauriRuntime()) {
    try {
      const loadedSettings = await loadAppSettings();
      settings.update(current => ({
        ...current,
        ...loadedSettings
      }));

      // Update active job with saved translation instruction if available
      if (loadedSettings.savedTranslationInstruction) {
        const current = get(activeJob);
        replaceActiveJob(updateJob(current, {
          form: {
            ...current.form,
            translationInstruction: loadedSettings.savedTranslationInstruction
          }
        }));
      }

      return loadedSettings.apiKey;
    } catch (error) {
      recordProcessError('Restore API key', error, 'Không thể khôi phục API key đã lưu.');
      return '';
    }
  }

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
    downloadingModelProgress.set(100);
    statusMessage.set(`Whisper model ${modelId} đã sẵn sàng.`);
  } catch (error) {
    recordProcessError('Download Whisper model', error, `Tải Whisper model ${modelId} thất bại.`);
  } finally {
    downloadingModelId.set(null);
    downloadingModelProgress.set(0);
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

function mergeTranslations(
  sourceSegments: SubtitleSegment[],
  translatedSegments: SubtitleSegment[]
): SubtitleSegment[] {
  return sourceSegments.map((segment, index) => ({
    ...segment,
    translatedText: translatedSegments[index]?.translatedText ?? segment.translatedText
  }));
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

    // Reuse existing normalized mp3 if available, skip re-normalization
    let normalizedAudioPath: string;
    const existingAudioPath = job.normalizedAudioPath;
    let canReuseAudio = false;

    if (existingAudioPath) {
      try {
        canReuseAudio = await pathExists(existingAudioPath);
      } catch {
        canReuseAudio = false;
      }
    }

    if (canReuseAudio && existingAudioPath) {
      normalizedAudioPath = existingAudioPath;
      job = withPhase(job, 'preprocess', 22, 'Sử dụng file audio đã normalize từ lần trước.');
      replaceActiveJob(job);
      // Don't save running jobs to history
    } else {
      job = withPhase(job, 'preprocess', 22, 'Đang normalize audio sang mp3.');
      replaceActiveJob(job);
      // Don't save running jobs to history

      const durationSeconds = job.inspection?.durationSeconds ?? 1;
      normalizedAudioPath = await normalizeAudio(
        job.id,
        job.form.inputPath,
        job.form.selectedAudioTrack,
        '',
        durationSeconds
      );
    }

    throwIfCancelled();

    job = updateJob(job, {
      normalizedAudioPath
    });

    let sourceSegments: SubtitleSegment[] = [];
    let translatedSegments: SubtitleSegment[] = [];

    if (job.form.processingMode === 'whisper_translate') {
      // whisper-rs is bundled; only a downloaded model file is needed.
      const modelReady = job.form.whisperModelId;
      if (!modelReady) {
        throw new Error('Chưa chọn Whisper model. Hãy chọn model trước khi dùng chế độ này.');
      }

      job = withPhase(job, 'transcribe', 45, 'Whisper đang tạo transcript có timestamp.');
      replaceActiveJob(job);
      // Don't save running jobs to history

      sourceSegments = await transcribeWithWhisper({
        jobId: job.id,
        audioPath: normalizedAudioPath,
        sourceLanguage: job.form.sourceLanguage,
        modelId: job.form.whisperModelId,
        computeMode: job.form.computeMode
      });

      throwIfCancelled();

      job = withPhase(job, 'translate', 70, 'Gemini đang dịch subtitle sang tiếng Việt.');
      replaceActiveJob(job);
      // Don't save running jobs to history

      translatedSegments = await translateSegmentsWithGemini({
        apiKey,
        modelId: job.form.geminiModelId,
        translationInstruction: job.form.translationInstruction,
        segments: sourceSegments
      });
      translatedSegments = mergeTranslations(sourceSegments, translatedSegments);
    } else {
      job = withPhase(
        job,
        'translate',
        60,
        'Gemini direct audio đang xử lý transcript và translation theo chế độ experimental.'
      );
      replaceActiveJob(job);
      // Don't save running jobs to history

      sourceSegments = await transcribeDirectWithGemini({
        apiKey,
        modelId: job.form.geminiModelId,
        audioPath: normalizedAudioPath,
        sourceLanguage: job.form.sourceLanguage,
        translationInstruction: job.form.translationInstruction
      });
      throwIfCancelled();
      translatedSegments = [...sourceSegments];
    }

    job = updateJob(
      withPhase(job, 'ready', 100, 'Subtitle đã sẵn sàng để chỉnh sửa và export.'),
      {
        status: 'completed',
        sourceSegments,
        translatedSegments
      }
    );

    replaceActiveJob(job);
    // Save to history only when completed successfully
    history.update((items) => upsertHistoryEntry(items, job));
  } catch (error) {
    const isCancelled = error instanceof Error && error.message === 'Job đã bị hủy bởi người dùng.';
    const message = errorMessage(error, 'Pipeline thất bại.');
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
}

export function clearHistory(): void {
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
    const failed = withPhase(get(activeJob), 'failed', get(activeJob).progress, errorMessage(error, 'Render video thất bại.'));
    replaceActiveJob(failed);
    // Don't save failed jobs to history
    recordProcessError('Render video', error, 'Render video thất bại.');
  } finally {
    busy.set(false);
  }
}
