import type {
  AppSettings,
  GeminiModelOption,
  JobFormState,
  SubtitleStyle,
  ViewKey,
  WhisperModelOption
} from '$lib/types';

export const APP_STORAGE_KEY = 'easyvietsub/app-state';
export const MAX_HISTORY_ENTRIES = 50;
export const MAX_LOG_ENTRIES = 100;
export const PROJECT_SCHEMA_VERSION = 1;

// Timing constants (milliseconds)
export const PERSIST_DEBOUNCE_MS = 100;
export const MODEL_DOWNLOAD_PROGRESS_THROTTLE_MS = 200;
export const PIPELINE_TIMER_INTERVAL_MS = 1000;

// Progress values
export const PROGRESS_COMPLETE = 100;
export const SUBTITLE_UPDATE_DEBOUNCE_MS = 300;

export const VIEW_LABELS: Record<ViewKey, string> = {
  translate: 'Translate',
  instruction: 'Instruction',
  render: 'Render',
  history: 'History',
  log: 'Log',
  settings: 'Settings'
};

export const SOURCE_LANGUAGE_OPTIONS = [
  { value: 'en', label: 'English' },
  { value: 'ja', label: 'Japanese' },
  { value: 'ko', label: 'Korean' },
  { value: 'zh', label: 'Chinese' },
  { value: 'fr', label: 'French' },
  { value: 'de', label: 'German' },
  { value: 'es', label: 'Spanish' },
  { value: 'ru', label: 'Russian' },
  { value: 'th', label: 'Thai' }
];

export const DEFAULT_STYLE: SubtitleStyle = {
  fontFamily: 'Arial',
  fontSize: 20,
  textColor: '#ffffff',
  outlineColor: '#111827',
  outlineWidth: 2,
  backgroundColor: '#000000b3',
  lineSpacing: 10,
  marginX: 48,
  marginY: 40,
  bold: true,
  italic: false,
  position: 'bottom'
};

export const DEFAULT_FORM: JobFormState = {
  inputPath: '',
  inputKind: 'video',
  sourceLanguage: 'en',
  processingMode: 'whisper_translate',
  computeMode: 'auto',
  selectedAudioTrack: 0,
  whisperModelId: 'small',
  geminiModelId: 'gemini-2.5-flash-lite',
  translationInstruction: `
Bạn là biên dịch viên phim chuyên nghiệp. Khi người dùng cung cấp transcript hoặc phụ đề gốc, nhiệm vụ của bạn là tái tạo lời thoại tiếng Việt sao cho khán giả cảm nhận đúng cảm xúc, tính cách nhân vật và nhịp điệu cảnh phim — không chỉ hiểu nghĩa.

---

## HÀNH VI MẶC ĐỊNH

Trước khi dịch, hãy tự động thực hiện theo thứ tự:

1. Đọc lướt toàn bộ nội dung được cung cấp (bao gồm phần ngữ cảnh đã dịch trước, nếu có) để nắm: tình huống, nhân vật, cảm xúc tổng thể, thuật ngữ và tên riêng.
2. Dịch từng dòng thoại theo ngữ cảnh đã nắm, đúng chế độ đầu ra người dùng yêu cầu.
3. Trước khi xuất kết quả, tự kiểm tra: xưng hô, thuật ngữ, giọng nhân vật, độ dài dòng.

---

## QUY TẮC BẮT BUỘC

### 1. Nhân vật & xưng hô
- Giữ đúng giới tính, địa vị, mối quan hệ nhân vật xuyên suốt.
- Hệ thống xưng hô (anh/em/mày/tao/ngài/hắn...) phải phản ánh đúng bối cảnh.
- Nhân vật có giọng đặc trưng (cộc lốc, lịch sự, đanh đá...) phải giữ nhất quán từ đầu đến cuối.

### 2. Tên riêng & thuật ngữ
- Tên người, địa danh, thương hiệu: giữ nguyên, không dịch.
- Thuật ngữ đặc thù: dịch nghĩa + giữ nguyên gốc trong ngoặc đơn.
  Ví dụ: "đặc vụ (agent)", "căn cứ (base)"

### 3. Tiếng lóng, humor, chửi thề
Không dịch nghĩa đen. Phân tích ngữ cảnh và tìm cách diễn đạt tương đương về sắc thái và cảm xúc trong tiếng Việt đương đại.

---

## CHẾ ĐỘ ĐẦU RA
- Không được lặp lại nội dung trong cùng 1 dòng, nếu lặp lại có thể dùng x2, x3...
- Mỗi dòng thoại = 1–2 dòng, tối đa ~42 ký tự/dòng.
- Giữ dấu câu tự nhiên, không viết tắt tùy tiện.
- Chỉ xuất lời thoại — KHÔNG xuất chú thích âm thanh.

---

## KIỂM TRA TRƯỚC KHI XUẤT KẾT QUẢ

Trước mỗi lần trả lời, tự kiểm tra nội bộ:
- [ ] Xưng hô nhất quán, đúng giới tính và quan hệ nhân vật
- [ ] Thuật ngữ: cùng từ → cùng cách dịch
- [ ] Giọng từng nhân vật nhất quán từ đầu đến cuối
- [ ] Chú thích âm thanh đã được lược bỏ hoàn toàn

---

## KHI DỊCH THEO PHẦN (CHUNKED)

Khi nội dung được chia thành nhiều phần để dịch:
- Phần "PREVIOUSLY TRANSLATED" (nếu có) là ngữ cảnh đã dịch trước đó — dùng để nắm giọng nhân vật, cách xưng hô và thuật ngữ đã chọn. KHÔNG đưa phần này vào output.
- Giữ đúng cách xưng hô, thuật ngữ và giọng nhân vật đã dùng trong phần context trước.
- Nếu nhân vật mới xuất hiện, suy luận quan hệ từ ngữ cảnh hiện có.

---

## XỬ LÝ TÌNH HUỐNG ĐẶC BIỆT

- Nếu một câu thoại mơ hồ hoặc có thể hiểu nhiều nghĩa: chọn nghĩa phù hợp nhất với ngữ cảnh cảnh phim, không hỏi lại người dùng.
- Nếu gặp từ/khái niệm không có tương đương tiếng Việt: giữ nguyên tiếng Anh và thêm chú thích ngắn trong ngoặc đơn.
- Nếu người dùng không cung cấp thông tin về nhân vật: suy luận giới tính và quan hệ từ ngữ cảnh, ghi chú giả định nếu không chắc.
  `
};

export const DEFAULT_SETTINGS: AppSettings = {
  apiKey: '',
  outputDirectory: '',
  lastOpenedProjectPath: '',
  savedTranslationInstruction: ''
};

export const WHISPER_MODELS: WhisperModelOption[] = [
  {
    id: 'small',
    label: 'small',
    filename: 'ggml-small.bin',
    sizeBytes: 488 * 1024 * 1024,
    description: 'Độ chính xác cao hơn với chi phí inference lớn hơn.',
    downloaded: false
  },
  {
    id: 'medium',
    label: 'medium',
    filename: 'ggml-medium.bin',
    sizeBytes: 1530 * 1024 * 1024,
    description: 'Phù hợp media dài hoặc âm thanh khó.',
    downloaded: false
  },
  {
    id: 'large',
    label: 'large',
    filename: 'ggml-large-v3.bin',
    sizeBytes: 3090 * 1024 * 1024,
    description: 'Chất lượng tốt nhất nhưng đòi hỏi tài nguyên cao.',
    downloaded: false
  }
];

export const FALLBACK_GEMINI_MODELS: GeminiModelOption[] = [
  {
    id: 'gemini-3.1-flash-lite-preview',
    label: 'Gemini 3.1 Flash Lite Preview',
    description: 'Preview inference. ID duoc suy luan theo convention ten model.'
  },
  {
    id: 'gemini-2.5-flash-lite',
    label: 'Gemini 2.5 Flash Lite',
    description: 'Nhanh, chi phi thap, phu hop subtitle workflow.'
  },
  {
    id: 'gemini-2.5-flash',
    label: 'Gemini 2.5 Flash',
    description: 'Can bang giua toc do va chat luong dau ra.'
  },
  {
    id: 'gemini-3-flash-preview',
    label: 'Gemini 3 Flash Preview',
    description: 'Preview model cho luong subtitle can latency thap.'
  }
];
