export type ViewKey = 'translate' | 'instruction' | 'render' | 'history' | 'log' | 'settings';

export type InputKind = 'video' | 'audio';

export type ProcessingMode = 'whisper_translate' | 'gemini_direct';

export type ComputeMode = 'auto' | 'cpu' | 'hardware';

export type JobPhase =
  | 'idle'
  | 'inspect'
  | 'preprocess'
  | 'download_model'
  | 'transcribe'
  | 'translate'
  | 'ready'
  | 'render'
  | 'complete'
  | 'failed'
  | 'cancelled';

export type JobStatus = 'draft' | 'running' | 'completed' | 'failed' | 'cancelled' | 'partial';

export interface AudioTrackInfo {
  index: number;
  codec: string;
  channels: number;
  language?: string | null;
  title: string;
}

export interface MediaInspection {
  path: string;
  kind: InputKind;
  durationSeconds: number;
  fileSizeBytes: number;
  audioTracks: AudioTrackInfo[];
  sampleRate?: number | null;
}

export interface WhisperModelOption {
  id: string;
  label: string;
  filename: string;
  sizeBytes: number;
  description: string;
  downloaded: boolean;
}

export interface GeminiModelOption {
  id: string;
  label: string;
  description: string;
  experimental?: boolean;
}

export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface SubtitleSegment {
  id: string;
  startMs: number;
  endMs: number;
  sourceText: string;
  translatedText: string;
}

export interface TranslationResult {
  segments: SubtitleSegment[];
  tokenUsage: TokenUsage;
}

export interface SubtitleStyle {
  fontFamily: string;
  fontSize: number;
  textColor: string;
  outlineColor: string;
  outlineWidth: number;
  backgroundColor: string;
  lineSpacing: number;
  marginX: number;
  marginY: number;
  bold: boolean;
  italic: boolean;
  position: 'bottom' | 'center' | 'top';
}

export interface JobFormState {
  inputPath: string;
  inputKind: InputKind;
  sourceLanguage: string;
  processingMode: ProcessingMode;
  computeMode: ComputeMode;
  selectedAudioTrack: number;
  whisperModelId: string;
  geminiModelId: string;
  translationInstruction: string;
}

export interface RuntimeCapabilities {
  os: string;
  ffmpegAvailable: boolean;
  ffprobeAvailable: boolean;
  localFfmpegInstalled: boolean;
  localFfprobeInstalled: boolean;
  hardwareAccelerationAvailable: boolean;
  detectedAccelerators: string[];
}

export interface JobRecord {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  status: JobStatus;
  phase: JobPhase;
  progress: number;
  etaSeconds?: number | null;
  message: string;
  inspection?: MediaInspection | null;
  sourceSegments: SubtitleSegment[];
  translatedSegments: SubtitleSegment[];
  form: JobFormState;
  style: SubtitleStyle;
  outputs: {
    sourceSrtPath?: string | null;
    translatedSrtPath?: string | null;
    renderedVideoPath?: string | null;
  };
  tokenUsage?: TokenUsage;
}

export interface PipelineProgressEvent {
  jobId: string;
  phase: string;
  progress: number;
  message: string;
  etaSeconds?: number | null;
}

export interface ModelDownloadProgressEvent {
  modelId: string;
  progress: number;
  downloadedBytes: number;
  totalBytes: number;
}

export interface RuntimeDownloadProgressEvent {
  stage: string;
  progress: number;
  downloadedBytes: number;
  totalBytes: number;
  message: string;
}

export interface ProjectSnapshot {
  version: number;
  exportedAt: string;
  job: JobRecord;
}

export interface AppSettings {
  apiKey: string;
  outputDirectory: string;
  lastOpenedProjectPath: string;
  savedTranslationInstruction: string;
}

export interface ProcessLogEntry {
  id: string;
  createdAt: string;
  context: string;
  message: string;
  detail?: string | null;
  stack?: string | null;
  jobId?: string | null;
  jobName?: string | null;
  phase?: string | null;
  progress?: number | null;
}
