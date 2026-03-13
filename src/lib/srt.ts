import type { SubtitleSegment } from '$lib/types';
import { formatTimestamp } from '$lib/utils';

export function segmentsToSrt(segments: SubtitleSegment[], useTranslatedText: boolean): string {
  return segments
    .map((segment, index) => {
      const text = useTranslatedText ? segment.translatedText : segment.sourceText;

      return [
        String(index + 1),
        `${formatTimestamp(segment.startMs)} --> ${formatTimestamp(segment.endMs)}`,
        (text ?? '').trim() || '(empty)',
        ''
      ].join('\n');
    })
    .join('\n');
}

/**
 * Parse SRT content into subtitle segments
 * @param content - Raw SRT file content
 * @returns Array of SubtitleSegment objects
 * @throws Error if parsing fails
 */
export function parseSrt(content: string): SubtitleSegment[] {
  const segments: SubtitleSegment[] = [];
  const lines = content.split(/\r?\n/);
  
  let currentIndex = 0;
  let segmentCounter = 0;
  
  while (currentIndex < lines.length) {
    // Skip empty lines
    if (lines[currentIndex].trim() === '') {
      currentIndex++;
      continue;
    }
    
    // Read sequence number
    const sequenceNum = lines[currentIndex].trim();
    if (!/^\d+$/.test(sequenceNum)) {
      currentIndex++;
      continue;
    }
    currentIndex++;
    
    // Read timestamp line
    if (currentIndex >= lines.length) break;
    const timestampLine = lines[currentIndex].trim();
    const timestampMatch = timestampLine.match(/^(\d{2}:\d{2}:\d{2},\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2},\d{3})$/);
    
    if (!timestampMatch) {
      currentIndex++;
      continue;
    }
    
    const startMs = parseTimestamp(timestampMatch[1]);
    const endMs = parseTimestamp(timestampMatch[2]);
    currentIndex++;
    
    // Read text lines until next empty line
    const textLines: string[] = [];
    while (currentIndex < lines.length && lines[currentIndex].trim() !== '') {
      textLines.push(lines[currentIndex]);
      currentIndex++;
    }
    
    const text = textLines.join('\n').trim();
    
    // Create segment with unique ID using counter to avoid collisions
    segments.push({
      id: `seg-${Date.now()}-${segmentCounter++}-${crypto.randomUUID().substring(0, 8)}`,
      startMs,
      endMs,
      sourceText: text,
      translatedText: text // Initially set translated text same as source
    });
  }
  
  return segments;
}

/**
 * Parse SRT timestamp to milliseconds
 * @param timestamp - Format: HH:MM:SS,mmm
 * @returns Milliseconds
 */
function parseTimestamp(timestamp: string): number {
  const parts = timestamp.split(':');
  const hours = parseInt(parts[0], 10);
  const minutes = parseInt(parts[1], 10);
  const secondsParts = parts[2].split(',');
  const seconds = parseInt(secondsParts[0], 10);
  const milliseconds = parseInt(secondsParts[1], 10);
  
  return hours * 3600000 + minutes * 60000 + seconds * 1000 + milliseconds;
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
