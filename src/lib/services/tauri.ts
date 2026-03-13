import type {
  AppSettings,
  GeminiModelOption,
  JobRecord,
  MediaInspection,
  ModelDownloadProgressEvent,
  PipelineProgressEvent,
  ProjectSnapshot,
  RuntimeDownloadProgressEvent,
  RuntimeCapabilities,
  SubtitleSegment,
  TranslationResult,
  WhisperModelOption
} from '$lib/types';
import { invokeTauri } from './tauri-commands';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function openPathSelection(
  directory = false,
  filters?: { name: string; extensions: string[] }[]
): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({
    directory,
    multiple: false,
    filters
  });

  if (!selected || Array.isArray(selected)) {
    return null;
  }

  return selected;
}

export async function savePathSelection(defaultPath: string): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { save } = await import('@tauri-apps/plugin-dialog');
  return save({
    defaultPath
  });
}

export async function writeTextContent(path: string, content: string): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error('Tauri runtime is required to write files.');
  }

  const { writeTextFile } = await import('@tauri-apps/plugin-fs');
  await writeTextFile(path, content);
}

export async function pathExists(path: string): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { exists } = await import('@tauri-apps/plugin-fs');
  return exists(path);
}

export async function readTextContent(path: string): Promise<string> {
  if (!isTauriRuntime()) {
    throw new Error('Tauri runtime is required to read files.');
  }

  const { readTextFile } = await import('@tauri-apps/plugin-fs');
  return readTextFile(path);
}

export async function detectRuntimeCapabilities(): Promise<RuntimeCapabilities> {
  if (!isTauriRuntime()) {
    return {
      os: 'browser',
      ffmpegAvailable: false,
      ffprobeAvailable: false,
      localFfmpegInstalled: false,
      localFfprobeInstalled: false,
      hardwareAccelerationAvailable: false,
      detectedAccelerators: []
    };
  }

  return invokeTauri('detect_runtime_capabilities');
}

export async function installLocalFfmpegRuntime(): Promise<RuntimeCapabilities> {
  return invokeTauri('install_local_ffmpeg_runtime');
}

export async function removeLocalFfmpegRuntime(): Promise<RuntimeCapabilities> {
  return invokeTauri('remove_local_ffmpeg_runtime');
}

export async function listWhisperModels(): Promise<WhisperModelOption[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  return invokeTauri('list_whisper_models');
}

export async function downloadWhisperModel(modelId: string): Promise<WhisperModelOption> {
  return invokeTauri('download_whisper_model', { modelId });
}

export async function removeWhisperModel(modelId: string): Promise<WhisperModelOption> {
  return invokeTauri('remove_whisper_model', { modelId });
}

export async function inspectMedia(filePath: string): Promise<MediaInspection> {
  return invokeTauri('inspect_media', { filePath });
}

export async function cleanupJobCache(jobId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  return invokeTauri('cleanup_job_cache', { jobId });
}

export async function getCacheSize(): Promise<number> {
  if (!isTauriRuntime()) return 0;
  return invokeTauri('get_cache_size');
}

export async function cleanupAllCache(): Promise<void> {
  if (!isTauriRuntime()) return;
  return invokeTauri('cleanup_all_cache');
}

export async function cleanupOldCache(days: number): Promise<number> {
  if (!isTauriRuntime()) return 0;
  return invokeTauri('cleanup_old_cache', { days });
}

export async function transcribeWithWhisper(payload: {
  jobId: string;
  audioPath: string;
  trackIndex: number;
  sourceLanguage: string;
  modelId: string;
  computeMode: string;
}): Promise<SubtitleSegment[]> {
  return invokeTauri('transcribe_with_whisper', {
    jobId: payload.jobId,
    payload: {
      audioPath: payload.audioPath,
      trackIndex: payload.trackIndex,
      sourceLanguage: payload.sourceLanguage,
      modelId: payload.modelId,
      computeMode: payload.computeMode
    }
  });
}

export async function translateSegmentsWithGemini(payload: {
  jobId: string;
  apiKey: string;
  modelId: string;
  translationInstruction: string;
  segments: SubtitleSegment[];
}): Promise<TranslationResult> {
  return invokeTauri('translate_segments_with_gemini', { payload });
}

export async function transcribeDirectWithGemini(payload: {
  jobId: string;
  apiKey: string;
  modelId: string;
  audioPath: string;
  trackIndex: number;
  durationSeconds: number;
  sourceLanguage: string;
  translationInstruction: string;
}): Promise<TranslationResult> {
  return invokeTauri('transcribe_direct_with_gemini', { payload });
}

export async function fetchSupportedGeminiModels(apiKey: string): Promise<GeminiModelOption[]> {
  return invokeTauri('fetch_supported_gemini_models', { apiKey });
}

export async function renderHardSubtitle(payload: {
  jobId: string;
  inputPath: string;
  outputPath: string;
  subtitleContent: string;
  style: JobRecord['style'];
  durationSeconds: number;
}): Promise<string> {
  return invokeTauri('render_hard_subtitle', {
    jobId: payload.jobId,
    durationSeconds: payload.durationSeconds,
    payload: {
      inputPath: payload.inputPath,
      outputPath: payload.outputPath,
      subtitleContent: payload.subtitleContent,
      style: payload.style
    }
  });
}

export async function exportAssSubtitle(payload: {
  path: string;
  segments: SubtitleSegment[];
  style: JobRecord['style'];
}): Promise<void> {
  return invokeTauri('export_ass_subtitle', {
    path: payload.path,
    segments: payload.segments,
    style: payload.style
  });
}

export async function cancelPipelineProcess(jobId: string): Promise<void> {
  return invokeTauri('cancel_pipeline_process', { jobId });
}

export async function loadAppSettings(): Promise<AppSettings> {
  return invokeTauri('load_app_settings');
}

export async function saveProjectSnapshot(payload: {
  path: string;
  snapshot: ProjectSnapshot;
}): Promise<void> {
  return invokeTauri('save_project_snapshot', payload);
}

export async function loadProjectSnapshot(payload: { path: string }): Promise<ProjectSnapshot> {
  return invokeTauri('load_project_snapshot', payload);
}

export async function persistAppSettings(settings: AppSettings): Promise<void> {
  await invokeTauri('persist_app_settings', { settings });
}

export async function listenToPipelineProgress(
  handler: (event: PipelineProgressEvent) => void
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<PipelineProgressEvent>('pipeline-progress', (event) => {
    handler(event.payload);
  });

  return unlisten;
}

export async function listenToModelDownloadProgress(
  handler: (event: ModelDownloadProgressEvent) => void
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<ModelDownloadProgressEvent>('model-download-progress', (event) => {
    handler(event.payload);
  });

  return unlisten;
}

export async function listenToRuntimeDownloadProgress(
  handler: (event: RuntimeDownloadProgressEvent) => void
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<RuntimeDownloadProgressEvent>('runtime-download-progress', (event) => {
    handler(event.payload);
  });

  return unlisten;
}

export async function getAssetUrl(filePath: string): Promise<string> {
  // Return empty string for invalid paths
  if (!filePath || filePath.trim() === '') {
    console.warn('[getAssetUrl] Empty or invalid file path');
    return '';
  }

  if (!isTauriRuntime()) {
    console.warn('[getAssetUrl] Not running in Tauri, returning original path');
    return filePath;
  }

  const { convertFileSrc } = await import('@tauri-apps/api/core');
  const assetUrl = convertFileSrc(filePath);
  console.log('[getAssetUrl] Converted:', filePath, '→', assetUrl);
  return assetUrl;
}

/**
 * Convert a local file path to a stream:// URL that supports HTTP Range requests.
 * This is needed for large video files where the asset:// protocol doesn't support seeking.
 */
export function getStreamUrl(filePath: string): string {
  if (!filePath || filePath.trim() === '') {
    return '';
  }

  if (!isTauriRuntime()) {
    return filePath;
  }

  // Encode the file path for use in URL
  const encoded = encodeURIComponent(filePath);
  return `stream://localhost/${encoded}`;
}
