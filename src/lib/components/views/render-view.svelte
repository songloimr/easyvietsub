<script lang="ts">
  import PreviewCanvas from '$lib/components/preview-canvas.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import Field from '$lib/components/ui/field.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import {
    activeJob,
    busy,
    inspection,
    renderCurrentVideo,
    settings,
    updateStyle
  } from '$lib/stores/app-store';
  import { openPathSelection, savePathSelection } from '$lib/services/tauri';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Video from '@lucide/svelte/icons/video';

  let canRender = $derived(!!$activeJob.form.inputPath && $activeJob.translatedSegments.length > 0 && !$busy);
  let renderIssues = $derived((() => {
    const issues: string[] = [];
    if (!$activeJob.form.inputPath) issues.push('Chưa chọn file media.');
    if ($activeJob.translatedSegments.length === 0) issues.push('Chưa có subtitle để render.');
    if (!$settings.outputDirectory) issues.push('Chưa chọn thư mục output.');
    const { fontSize, outlineWidth, marginX, marginY, lineSpacing, textColor, outlineColor, fontFamily } = $activeJob.style;
    if (!fontFamily.trim()) issues.push('Font family không được để trống.');
    if (fontSize < 14 || fontSize > 80) issues.push('Font size phải từ 14–80px.');
    if (outlineWidth < 0 || outlineWidth > 8) issues.push('Outline width phải từ 0–8.');
    if (marginX < 0 || marginX > 160) issues.push('Horizontal margin phải từ 0–160.');
    if (marginY < 0 || marginY > 160) issues.push('Vertical margin phải từ 0–160.');
    if (lineSpacing < 0 || lineSpacing > 32) issues.push('Line spacing phải từ 0–32.');
    if (!/^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$/.test(textColor) && !textColor.startsWith('rgba') && !textColor.startsWith('rgb')) {
      issues.push('Text color không hợp lệ (dùng #rrggbb hoặc #rrggbbaa).');
    }
    if (outlineWidth > 0 && !/^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$/.test(outlineColor) && !outlineColor.startsWith('rgba') && !outlineColor.startsWith('rgb')) {
      issues.push('Outline color không hợp lệ.');
    }
    return issues;
  })());

  function colorPreviewStyle(color: string): string {
    return `background:${color || 'transparent'}`;
  }

  function joinPath(base: string, filename: string): string {
    if (!base.trim()) return filename;
    return `${base.replace(/[\\/]+$/, '')}/${filename}`;
  }

  async function handleChooseOutputDirectory(): Promise<void> {
    const path = await openPathSelection(true);
    if (!path) return;
    settings.update((current) => ({
      ...current,
      outputDirectory: path
    }));
  }

  async function handleRenderVideo(): Promise<void> {
    if (!$activeJob.form.inputPath || $activeJob.translatedSegments.length === 0) return;
    const suggestedName = `${($activeJob.name || 'subtitle').replace(/\.[^.]+$/, '')}.burned.mp4`;
    const outputPath = await savePathSelection(joinPath($settings.outputDirectory, suggestedName));
    if (!outputPath) return;
    await renderCurrentVideo(outputPath);
  }
</script>

<div class="view-enter space-y-4">
  <!-- Preview -->
  <Card class="border bg-card overflow-hidden">
    <div class="p-4">
      <h2 class="text-base font-semibold mb-3">Preview</h2>
      <div class="rounded-lg border bg-background/60 p-2">
        <PreviewCanvas 
          style={$activeJob.style} 
          segments={$activeJob.translatedSegments}
          mediaPath={$activeJob.form.inputPath || null}
          mediaKind={$activeJob.form.inputKind}
          mediaDuration={$inspection?.durationSeconds ?? 0}
        />
      </div>
    </div>
  </Card>

  <div class="grid gap-4 lg:grid-cols-2">
    <!-- Render Controls -->
    <Card class="border bg-card">
      <div class="p-4">
        <h2 class="text-sm font-semibold mb-3">Render video</h2>

        <div class="space-y-3">
          <Field label="Output directory">
            <div class="flex items-center gap-1.5">
              <Input value={$settings.outputDirectory} disabled class="flex-1 text-xs" />
              <Button
                variant="outline"
                size="icon"
                title="Browse output"
                ariaLabel="Browse output"
                onclick={handleChooseOutputDirectory}
              >
                <FolderOpen class="size-3.5" />
              </Button>
            </div>
          </Field>

          <Button class="w-full" disabled={!canRender || renderIssues.length > 0} onclick={handleRenderVideo} size="sm">
            <Video class="size-3.5" />
            Render
          </Button>

          {#if renderIssues.length > 0}
            <div class="rounded-md border border-amber-500/30 bg-amber-50/10 p-2.5 text-xs text-amber-700 dark:text-amber-400 space-y-0.5">
              {#each renderIssues as issue}
                <p>- {issue}</p>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </Card>

    <!-- Typography & Colors -->
    <Card class="border bg-card">
      <div class="p-4">
        <h2 class="text-sm font-semibold mb-3">Typography</h2>

        <div class="space-y-3">
          <div class="grid grid-cols-2 gap-2.5">
            <Field label="Font size">
              <Input
                type="number"
                value={$activeJob.style.fontSize}
                min={14}
                max={80}
                onchange={(event) => updateStyle('fontSize', Number(event.currentTarget.value))}
                class="h-8 text-xs"
              />
            </Field>
            <Field label="Line spacing">
              <Input
                type="number"
                value={$activeJob.style.lineSpacing}
                min={0}
                max={32}
                onchange={(event) => updateStyle('lineSpacing', Number(event.currentTarget.value))}
                class="h-8 text-xs"
              />
            </Field>
          </div>

          <div class="grid grid-cols-2 gap-2.5">
            <Field label="Text color">
              <div class="flex items-center gap-1.5">
                <span
                  class="size-7 shrink-0 rounded border"
                  style={colorPreviewStyle($activeJob.style.textColor)}
                ></span>
                <Input
                  value={$activeJob.style.textColor}
                  oninput={(event) => updateStyle('textColor', event.currentTarget.value)}
                  class="h-8 text-xs"
                />
              </div>
            </Field>
            <Field label="Outline color">
              <div class="flex items-center gap-1.5">
                <span
                  class="size-7 shrink-0 rounded border"
                  style={colorPreviewStyle($activeJob.style.outlineColor)}
                ></span>
                <Input
                  value={$activeJob.style.outlineColor}
                  oninput={(event) => updateStyle('outlineColor', event.currentTarget.value)}
                  class="h-8 text-xs"
                />
              </div>
            </Field>
          </div>

          <div class="grid grid-cols-2 gap-2.5">
            <Field label="Position">
              <Select
                value={$activeJob.style.position}
                options={[
                  { value: 'bottom', label: 'Bottom' },
                  { value: 'center', label: 'Center' },
                  { value: 'top', label: 'Top' }
                ]}
                onchange={(event) =>
                  updateStyle('position', event.currentTarget.value as 'bottom' | 'center' | 'top')}
                class="h-8 text-xs"
              />
            </Field>
            <Field label="Outline width">
              <Input
                type="number"
                value={$activeJob.style.outlineWidth}
                min={0}
                max={8}
                onchange={(event) => updateStyle('outlineWidth', Number(event.currentTarget.value))}
                class="h-8 text-xs"
              />
            </Field>
          </div>

          <div class="grid grid-cols-2 gap-2.5">
            <Field label="Horizontal margin">
              <Input
                type="number"
                value={$activeJob.style.marginX}
                min={0}
                max={160}
                onchange={(event) => updateStyle('marginX', Number(event.currentTarget.value))}
                class="h-8 text-xs"
              />
            </Field>
            <Field label="Vertical margin">
              <Input
                type="number"
                value={$activeJob.style.marginY}
                min={0}
                max={160}
                onchange={(event) => updateStyle('marginY', Number(event.currentTarget.value))}
                class="h-8 text-xs"
              />
            </Field>
          </div>

          <div class="flex gap-2">
            <Button
              variant={$activeJob.style.bold ? 'default' : 'outline'}
              size="sm"
              onclick={() => updateStyle('bold', !$activeJob.style.bold)}
              class="flex-1 text-xs"
            >
              Bold
            </Button>
            <Button
              variant={$activeJob.style.italic ? 'default' : 'outline'}
              size="sm"
              onclick={() => updateStyle('italic', !$activeJob.style.italic)}
              class="flex-1 text-xs"
            >
              Italic
            </Button>
          </div>
        </div>
      </div>
    </Card>
  </div>
</div>
