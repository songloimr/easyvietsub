<script lang="ts">
	import type { SubtitleSegment, SubtitleStyle } from '$lib/types';
	import { getStreamUrl } from '$lib/services/tauri';

	interface Props {
		style: SubtitleStyle;
		segments: SubtitleSegment[];
		mediaPath: string | null;
		mediaKind: 'video' | 'audio';
		mediaDuration?: number;
	}

	let { style, segments, mediaPath, mediaKind, mediaDuration = 0 }: Props = $props();

	let currentTime = $state(0);
	let duration = $state(0);
	let paused = $state(true);
	let assetUrl = $state<string | null>(null);
	let videoError = $state<string | null>(null);
	let videoEl = $state<HTMLVideoElement | null>(null);
	let seeking = $state(false);
	let seekValue = $state(0);

	// Safe duration: prefer browser-reported duration, fallback to ffprobe mediaDuration
	const safeDuration = $derived.by(() => {
		if (Number.isFinite(duration) && duration > 0) return duration;
		if (Number.isFinite(mediaDuration) && mediaDuration > 0) return mediaDuration;
		return 0;
	});

	// Display time: show seekValue while dragging, otherwise currentTime
	const displayTime = $derived(seeking ? seekValue : currentTime);

	// Progress percentage for styling the range track fill
	const progress = $derived(safeDuration > 0 ? (displayTime / safeDuration) * 100 : 0);

	// Find active subtitle based on current time using binary search
	const activeSegment = $derived.by(() => {
		if (!segments || segments.length === 0) return null;
		const currentTimeMs = currentTime * 1000;
		
		// Binary search for segment containing current time
		let left = 0;
		let right = segments.length - 1;
		
		while (left <= right) {
			const mid = Math.floor((left + right) / 2);
			const segment = segments[mid];
			
			if (currentTimeMs < segment.startMs) {
				right = mid - 1;
			} else if (currentTimeMs > segment.endMs) {
				left = mid + 1;
			} else {
				// Found matching segment
				return segment;
			}
		}
		
		return null;
	});

	const activeText = $derived.by(() => {
		// If we have an active segment with translated text, show it
		if (activeSegment?.translatedText) {
			return activeSegment.translatedText;
		}
		
		// If video is loaded and we have segments, but no match - hide subtitle
		if (assetUrl && segments.length > 0) {
			return null;
		}
		
		// Fallback: show placeholder only when no video or no segments
		if (!assetUrl) {
			return 'Văn bản phụ đề tiếng Việt xuất hiện ở đây';
		}
		
		return null;
	});

	// Compute CSS styles for subtitle
	// Cache for text-shadow computation (expensive trigonometry)
	let shadowCache: { outlineWidth: number; outlineColor: string; result: string } | null = null;
	
	const subtitleCssStyle = $derived.by(() => {
		const {
			fontFamily,
			fontSize,
			textColor,
			outlineColor,
			outlineWidth,
			lineSpacing,
			marginX,
			marginY,
			bold,
			italic
		} = style;

		// Generate text-shadow for outline effect (8 directions per unit width)
		// Use cache if outlineWidth and outlineColor haven't changed
		let textShadow: string;
		if (
			shadowCache &&
			shadowCache.outlineWidth === outlineWidth &&
			shadowCache.outlineColor === outlineColor
		) {
			textShadow = shadowCache.result;
		} else {
			const steps = Math.max(8, Math.ceil(outlineWidth) * 8);
			const shadows: string[] = [];
			for (let i = 0; i < steps; i++) {
				const angle = (i * Math.PI * 2) / steps;
				const x = Math.cos(angle) * outlineWidth;
				const y = Math.sin(angle) * outlineWidth;
				shadows.push(`${x.toFixed(1)}px ${y.toFixed(1)}px 0 ${outlineColor}`);
			}
			textShadow = shadows.join(', ');
			shadowCache = { outlineWidth, outlineColor, result: textShadow };
		}

		return [
			`font-family: "${fontFamily}", sans-serif`,
			`font-size: ${fontSize}px`,
			`color: ${textColor}`,
			`font-weight: ${bold ? 'bold' : 'normal'}`,
			`font-style: ${italic ? 'italic' : 'normal'}`,
			`text-shadow: ${textShadow}`,
			`line-height: ${fontSize + lineSpacing}px`,
			`padding: ${marginY}px ${marginX}px`,
			`text-align: center`,
			`white-space: pre-wrap`,
			`word-break: break-word`,
			`max-width: 90%`
		].join('; ');
	});

	const positionAlign = $derived.by(() => {
		switch (style.position) {
			case 'top':
				return 'flex-start';
			case 'center':
				return 'center';
			case 'bottom':
			default:
				return 'flex-end';
		}
	});

	// Format time hh:mm:ss or mm:ss
	function formatTime(seconds: number): string {
		if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = Math.floor(seconds % 60);
		if (h > 0) {
			return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
		}
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	// Load stream URL when media path changes
	$effect(() => {
		if (mediaPath && mediaPath.trim() !== '' && mediaKind === 'video') {
			assetUrl = getStreamUrl(mediaPath);
		} else {
			assetUrl = null;
		}
	});
</script>

<div class="relative aspect-video overflow-hidden rounded-lg border border-border bg-card shadow-sm">
	<!-- Layer 1: Video or Gradient Background -->
	{#if assetUrl}
		<video
			bind:this={videoEl}
			src={assetUrl}
			muted
			bind:paused
			class="h-full w-full bg-black object-contain"
			preload="metadata"
			ontimeupdate={(e) => {
				const video = e.currentTarget;
				if (!seeking) {
					currentTime = video.currentTime;
				}
			}}
			ondurationchange={(e) => {
				const d = e.currentTarget.duration;
				if (Number.isFinite(d) && d > 0) {
					duration = d;
				}
			}}
			onloadedmetadata={(e) => {
				videoError = null;
				const d = e.currentTarget.duration;
				if (Number.isFinite(d) && d > 0) {
					duration = d;
				}
			}}
			onloadstart={() => {
				videoError = null;
			}}
			onerror={(e) => {
				const target = e.target as HTMLVideoElement;
				const error = target.error;
				const errorMsg = error 
					? `Video error: code=${error.code}, message=${error.message}` 
					: 'Unknown video error';
				videoError = errorMsg;
			}}
		>
			<track kind="captions" />
		</video>
	{:else}
		<div
			class="h-full w-full"
			style="background: linear-gradient(135deg, #1f2937 0%, #111827 50%, #0f172a 100%)"
		></div>
	{/if}

	<!-- Layer 2: Subtitle Overlay -->
	{#if activeText}
		<div
			class="pointer-events-none absolute inset-0 flex justify-center"
			style:align-items={positionAlign}
		>
			<p style={subtitleCssStyle}>
				{activeText}
			</p>
		</div>
	{/if}

	<!-- Layer 3: Seekbar Controls (always shown when video is loaded) -->
	{#if assetUrl}
		<div class="group absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/60 to-transparent px-4 pb-3 pt-8">
			<div class="flex items-center gap-3">
				<!-- Current Time -->
				<span class="text-sm font-medium tabular-nums text-white drop-shadow-lg">
					{formatTime(displayTime)}
				</span>

				<!-- Seekbar -->
				<input
					type="range"
					min={0}
					max={safeDuration || 1}
					step="any"
					value={displayTime}
					oninput={(e) => {
						seeking = true;
						seekValue = parseFloat(e.currentTarget.value);
					}}
					onchange={(e) => {
						const val = parseFloat(e.currentTarget.value);
						if (videoEl && Number.isFinite(val)) {
							videoEl.currentTime = val;
							currentTime = val;
						}
						seeking = false;
					}}
					class="seekbar flex-1"
					style:--progress="{progress}%"
				/>

				<!-- Duration -->
				<span class="text-sm font-medium tabular-nums text-white/80 drop-shadow-lg">
					{formatTime(safeDuration)}
				</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.seekbar {
		-webkit-appearance: none;
		appearance: none;
		height: 6px;
		border-radius: 9999px;
		background: linear-gradient(
			to right,
			rgba(255, 255, 255, 0.9) 0%,
			rgba(255, 255, 255, 0.9) var(--progress),
			rgba(255, 255, 255, 0.2) var(--progress),
			rgba(255, 255, 255, 0.2) 100%
		);
		outline: none;
		cursor: pointer;
	}

	.seekbar::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: white;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
		cursor: pointer;
	}

	.seekbar::-moz-range-track {
		height: 6px;
		border-radius: 9999px;
		background: rgba(255, 255, 255, 0.2);
	}

	.seekbar::-moz-range-progress {
		height: 6px;
		border-radius: 9999px;
		background: rgba(255, 255, 255, 0.9);
	}

	.seekbar::-moz-range-thumb {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: white;
		border: none;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
		cursor: pointer;
	}
</style>

