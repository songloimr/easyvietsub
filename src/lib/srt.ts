import type { SubtitleSegment } from '$lib/types';
import { formatTimestamp } from '$lib/utils';

export function segmentsToSrt(segments: SubtitleSegment[], useTranslatedText: boolean): string {
  return segments
    .filter(segment => {
      const text = useTranslatedText ? segment.translatedText : segment.sourceText;
      return (text ?? '').trim() !== '';
    })
    .map((segment, index) => {
      const text = useTranslatedText ? segment.translatedText : segment.sourceText;

      return [
        String(index + 1),
        `${formatTimestamp(segment.startMs)} --> ${formatTimestamp(segment.endMs)}`,
        (text ?? '').trim(),
        ''
      ].join('\n');
    })
    .join('\n');
}

export function validateSegments(segments: SubtitleSegment[]): string[] {
  const errors: string[] = [];

  for (let index = 0; index < segments.length; index += 1) {
    const current = segments[index];
    const previous = segments[index - 1];

    if (current.endMs <= current.startMs) {
      errors.push(`Segment ${index + 1} có end time phải lớn hơn start time.`);
    }

    if (previous && current.startMs < previous.endMs) {
      errors.push(`Segment ${index + 1} đang bị overlap với segment trước đó.`);
    }
  }

  return errors;
}
