<script lang="ts">
	import type { SubtitleSegment, SubtitleStyle } from '$lib/types';
	import { getAssetUrl } from '$lib/services/tauri';
	import { Slider } from '$lib/components/ui/slider/index.js';

	interface Props {
		style: SubtitleStyle;
		segments: SubtitleSegment[];
		mediaPath: string | null;
		mediaKind: 'video' | 'audio';
	}

	let { style, segments, mediaPath, mediaKind }: Props = $props();

	let currentTime = $state(0);
	let duration = $state(0);
	let paused = $state(true);
	let assetUrl = $state<string | null>(null);
	let seeking = $state(false);
	let videoError = $state<string | null>(null);

	// Find active subtitle based on current time
	const activeSegment = $derived.by(() => {
		if (!segments || segments.length === 0) return null;
		const currentTimeMs = currentTime * 1000;
		
		// Find segment that contains current time
		const segment = segments.find((s) => s.startMs <= currentTimeMs && currentTimeMs <= s.endMs);
		
		return segment ?? null;
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
		const steps = Math.max(8, Math.ceil(outlineWidth) * 8);
		const shadows: string[] = [];
		for (let i = 0; i < steps; i++) {
			const angle = (i * Math.PI * 2) / steps;
			const x = Math.cos(angle) * outlineWidth;
			const y = Math.sin(angle) * outlineWidth;
			shadows.push(`${x.toFixed(1)}px ${y.toFixed(1)}px 0 ${outlineColor}`);
		}

		return [
			`font-family: "${fontFamily}", sans-serif`,
			`font-size: ${fontSize}px`,
			`color: ${textColor}`,
			`font-weight: ${bold ? 'bold' : 'normal'}`,
			`font-style: ${italic ? 'italic' : 'normal'}`,
			`text-shadow: ${shadows.join(', ')}`,
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

	// Format time mm:ss
	function formatTime(seconds: number): string {
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	// Load asset URL when media path changes
	$effect(() => {
		// Check for non-empty mediaPath and video kind
		if (mediaPath && mediaPath.trim() !== '' && mediaKind === 'video') {
			getAssetUrl(mediaPath).then((url: string) => {
				assetUrl = url;
			}).catch(() => {
				assetUrl = null;
			});
		} else {
			assetUrl = null;
		}
	});
</script>

<div class="relative aspect-video overflow-hidden rounded-lg border border-border bg-card shadow-sm">
	<!-- Layer 1: Video or Gradient Background -->
	{#if assetUrl}
		<video
			src={assetUrl}
			muted
			bind:currentTime
			bind:duration
			bind:paused
			class="h-full w-full bg-black object-contain"
			preload="metadata"
			onloadstart={() => {
				videoError = null;
			}}
			onloadedmetadata={() => {
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
					{formatTime(currentTime)}
				</span>

				<!-- Seekbar Slider -->
				<Slider
					type="single"
					bind:value={currentTime}
					min={0}
					max={duration || 1}
					step={0.1}
					class="flex-1 [&>span:first-child]:h-1.5 [&>span:first-child]:bg-white/20 [&_[data-slider-range]]:bg-white [&_[data-slider-thumb]]:size-3.5 [&_[data-slider-thumb]]:border-white [&_[data-slider-thumb]]:bg-white [&_[data-slider-thumb]]:shadow-md"
				/>

				<!-- Duration -->
				<span class="text-sm font-medium tabular-nums text-white/80 drop-shadow-lg">
					{formatTime(duration)}
				</span>
			</div>
		</div>
	{/if}
</div>

