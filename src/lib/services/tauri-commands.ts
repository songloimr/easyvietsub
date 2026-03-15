/**
 * Type-safe Tauri command definitions.
 * This file provides compile-time type safety for all Tauri backend commands.
 */

import type {
  AppSettings,
  GeminiModelOption,
  JobRecord,
  MediaInspection,
  ProjectSnapshot,
  RuntimeCapabilities,
  SubtitleSegment,
  TranslationResult,
  WhisperModelOption
} from '$lib/types';

// ========================================
// Command Input/Output Type Definitions
// ========================================

export interface TauriCommands {
  // Runtime detection
  detect_runtime_capabilities: {
    input: undefined;
    output: RuntimeCapabilities;
  };

  // Runtime installation
  install_local_ffmpeg_runtime: {
    input: undefined;
    output: RuntimeCapabilities;
  };
  remove_local_ffmpeg_runtime: {
    input: undefined;
    output: RuntimeCapabilities;
  };

  // Whisper model management
  list_whisper_models: {
    input: undefined;
    output: WhisperModelOption[];
  };
  download_whisper_model: {
    input: { modelId: string };
    output: WhisperModelOption;
  };
  remove_whisper_model: {
    input: { modelId: string };
    output: WhisperModelOption;
  };

  // Media inspection
  inspect_media: {
    input: { filePath: string };
    output: MediaInspection;
  };

  // Cache management
  cleanup_job_cache: {
    input: { jobId: string };
    output: void;
  };
  get_cache_size: {
    input: undefined;
    output: number;
  };
  cleanup_all_cache: {
    input: undefined;
    output: void;
  };
  cleanup_old_cache: {
    input: { days: number };
    output: number;
  };

  // Transcription
  transcribe_with_whisper: {
    input: {
      jobId: string;
      payload: {
        audioPath: string;
        trackIndex: number;
        sourceLanguage: string;
        modelId: string;
        cpuOnly: boolean;
      };
    };
    output: SubtitleSegment[];
  };

  // Translation
  translate_segments_with_gemini: {
    input: {
      payload: {
        jobId: string;
        apiKey: string;
        modelId: string;
        translationInstruction: string;
        segments: SubtitleSegment[];
      };
    };
    output: TranslationResult;
  };

  transcribe_direct_with_gemini: {
    input: {
      payload: {
        jobId: string;
        apiKey: string;
        modelId: string;
        audioPath: string;
        trackIndex: number;
        durationSeconds: number;
        sourceLanguage: string;
        translationInstruction: string;
      };
    };
    output: TranslationResult;
  };

  // Gemini API
  fetch_supported_gemini_models: {
    input: { apiKey: string };
    output: GeminiModelOption[];
  };

  // Rendering
  render_hard_subtitle: {
    input: {
      jobId: string;
      durationSeconds: number;
      payload: {
        inputPath: string;
        outputPath: string;
        subtitleContent: string;
        style: JobRecord['style'];
      };
    };
    output: string;
  };

  // Export
  export_ass_subtitle: {
    input: {
      path: string;
      segments: SubtitleSegment[];
      style: JobRecord['style'];
    };
    output: void;
  };

  // Pipeline control
  cancel_pipeline_process: {
    input: { jobId: string };
    output: void;
  };

  // Settings
  load_app_settings: {
    input: undefined;
    output: AppSettings;
  };
  persist_app_settings: {
    input: { settings: AppSettings };
    output: void;
  };

  // Project snapshots
  save_project_snapshot: {
    input: {
      path: string;
      snapshot: ProjectSnapshot;
    };
    output: void;
  };
  load_project_snapshot: {
    input: { path: string };
    output: ProjectSnapshot;
  };
}

// ========================================
// Type-safe Tauri Invoke Function
// ========================================

/**
 * Type-safe wrapper around Tauri's invoke function.
 * Provides compile-time type checking for command names and their arguments.
 * 
 * @example
 * const models = await invokeTauri('list_whisper_models');
 * const result = await invokeTauri('transcribe_with_whisper', { jobId: '123', payload: {...} });
 */
// Lazily cached invoke function to avoid dynamic import on every call
let _invoke: typeof import('@tauri-apps/api/core').invoke | null = null;

async function getInvoke() {
  if (!_invoke) {
    const { invoke } = await import('@tauri-apps/api/core');
    _invoke = invoke;
  }
  return _invoke;
}

export async function invokeTauri<K extends keyof TauriCommands>(
  command: K,
  ...args: TauriCommands[K]['input'] extends undefined
    ? []
    : [TauriCommands[K]['input']]
): Promise<TauriCommands[K]['output']> {
  const invoke = await getInvoke();
  const input = args[0];
  return invoke<TauriCommands[K]['output']>(command, input);
}
