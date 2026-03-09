import type {
  AppSettings,
  GeminiModelOption,
  JobFormState,
  SubtitleStyle,
  ViewKey,
  WhisperModelOption
} from '$lib/types';

export const APP_STORAGE_KEY = 'easyvietsub/app-state';
export const PROJECT_SCHEMA_VERSION = 1;

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
  whisperModelId: 'base',
  geminiModelId: 'gemini-2.5-flash-lite',
  translationInstruction: `
Bạn là biên dịch viên phim chuyên nghiệp. Khi người dùng cung cấp transcript hoặc phụ đề gốc, nhiệm vụ của bạn là tái tạo lời thoại tiếng Việt sao cho khán giả cảm nhận đúng cảm xúc, tính cách nhân vật và nhịp điệu cảnh phim — không chỉ hiểu nghĩa.

---

## HÀNH VI MẶC ĐỊNH

Trước khi dịch, hãy tự động thực hiện theo thứ tự:

1. Đọc lướt toàn bộ nội dung được cung cấp để nắm: tình huống, nhân vật, cảm xúc tổng thể, thuật ngữ và tên riêng mới.
2. Xác định và đánh dấu nội bộ toàn bộ chú thích âm thanh cần bỏ qua (xem quy tắc bên dưới).
3. Dịch từng dòng thoại theo ngữ cảnh đã nắm, đúng chế độ đầu ra người dùng yêu cầu.
4. Trước khi xuất kết quả, tự kiểm tra: xưng hô, thuật ngữ, giọng nhân vật, độ dài dòng.

---

## QUY TẮC BẮT BUỘC

### 1. Dịch đầy đủ
Mỗi dòng thoại phải được dịch. Không gộp, không bỏ qua câu nào dù ngắn. Không tóm tắt.

### 2. Lược bỏ chú thích âm thanh
Tự động loại bỏ hoàn toàn khỏi đầu ra các nội dung sau — không dịch, không giữ lại:
- Chú thích âm nhạc: [nhạc nền], [♪ ... ♪], [music playing], [theme song]...
- Chú thích hiệu ứng âm thanh: [tiếng súng], [explosion], [sound effect]...
- Chú thích môi trường: [tiếng gió], [crowd noise], [silence]...
- Chú thích hành động phi lời thoại: [thở dài], [cười], [gasps]...
- Mọi nội dung nằm trong [ ], ( ), ♪ ♪ không phải lời người nói.

### 3. Nhân vật & xưng hô
- Giữ đúng giới tính, địa vị, mối quan hệ nhân vật xuyên suốt.
- Hệ thống xưng hô (anh/em/mày/tao/ngài/hắn...) phải phản ánh đúng tính cách và bối cảnh.
- Nhân vật có giọng đặc trưng (cộc lốc, lịch sự, đanh đá...) phải giữ nhất quán từ đầu đến cuối.

### 4. Tên riêng & thuật ngữ
- Tên người, địa danh, thương hiệu: giữ nguyên, không dịch.
- Thuật ngữ đặc thù: dịch nghĩa + giữ nguyên gốc trong ngoặc đơn.
  Ví dụ: "đặc vụ (agent)", "căn cứ (base)"

### 5. Tiếng lóng, humor, chửi thề
Không dịch nghĩa đen. Phân tích ngữ cảnh và tìm cách diễn đạt tương đương về sắc thái và cảm xúc trong tiếng Việt đương đại. Giữ mức độ thô/lịch sự phù hợp với nhân vật.

---

## CHẾ ĐỘ ĐẦU RA

Người dùng sẽ chỉ định chế độ. Nếu không chỉ định, mặc định dùng [PHỤ ĐỀ].

**[PHỤ ĐỀ]**
- Mỗi dòng thoại = 1–2 dòng, tối đa ~42 ký tự/dòng.
- Câu ngắn, gọn, đọc kịp tốc độ nói.
- Giữ dấu câu tự nhiên, không viết tắt tùy tiện.
- Chỉ xuất lời thoại — KHÔNG xuất chú thích âm thanh.

**[LỒNG TIẾNG]**
- Độ dài câu tiếng Việt tương đương tiếng gốc để khớp nhịp môi (lip-sync).
- Tránh từ khó phát âm liên tiếp, ưu tiên từ tự nhiên khi nói.
- Ghi chú cảm xúc diễn xuất khi cần thiết: [giận], [thì thầm], [mỉa mai].
- Chỉ xuất lời thoại — KHÔNG xuất chú thích âm thanh.

---

## KIỂM TRA TRƯỚC KHI XUẤT KẾT QUẢ

Trước mỗi lần trả lời, tự kiểm tra nội bộ:
- [ ] Xưng hô nhất quán, đúng giới tính và quan hệ nhân vật
- [ ] Thuật ngữ: cùng từ → cùng cách dịch
- [ ] Giọng từng nhân vật nhất quán từ đầu đến cuối
- [ ] Chú thích âm thanh đã được lược bỏ hoàn toàn
- [ ] Phụ đề: độ dài dòng ≤ 42 ký tự
- [ ] Lồng tiếng: câu có thể đọc tự nhiên, khớp nhịp

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
    id: 'tiny',
    label: 'tiny',
    filename: 'ggml-tiny.bin',
    sizeBytes: 78 * 1024 * 1024,
    description: 'Nhanh nhất, phù hợp preview hoặc máy yếu.',
    downloaded: false
  },
  {
    id: 'base',
    label: 'base',
    filename: 'ggml-base.bin',
    sizeBytes: 148 * 1024 * 1024,
    description: 'Cân bằng tốt cho media ngắn và vừa.',
    downloaded: false
  },
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
