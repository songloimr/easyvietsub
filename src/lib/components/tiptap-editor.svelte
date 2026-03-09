<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
	import { Editor } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import { Markdown } from 'tiptap-markdown';
	import Undo2 from '@lucide/svelte/icons/undo-2';
  import Redo2 from '@lucide/svelte/icons/redo-2';
  import BoldIcon from '@lucide/svelte/icons/bold';
  import ItalicIcon from '@lucide/svelte/icons/italic';
  import ListIcon from '@lucide/svelte/icons/list';
  import ListOrdered from '@lucide/svelte/icons/list-ordered';
  import Save from '@lucide/svelte/icons/save';
  import Check from '@lucide/svelte/icons/check';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Button from '$lib/components/ui/button.svelte';

  interface Props {
    value?: string;
    oninput?: (value: string) => void;
    onsave?: (value: string) => void;
    onrestore?: () => void;
    isSaved?: boolean;
  }

  let { value = '', oninput = () => {}, onsave = () => {}, onrestore, isSaved = true }: Props = $props();

  let element = $state<HTMLDivElement>();
  // Wrapper object pattern from official TipTap Svelte 5 docs:
  // Reassigning the whole object on every transaction forces Svelte 5 reactivity
  let editorState: { editor: Editor | null } = $state({ editor: null });
  let isInitializing = $state(true);
  // Flag to prevent $effect from calling setContent when the editor itself triggered the value change
  let isInternalUpdate = false;

  onMount(() => {
    editorState.editor = new Editor({
      element: element,
      extensions: [
        StarterKit.configure({
          heading: {
            levels: [2, 3],
          },
          blockquote: false,
          code: false,
          codeBlock: false,
          strike: false,
        }),
        Markdown.configure({
          html: false,
          transformCopiedText: true,
          transformPastedText: true,
        }),
      ],
      content: value || '<p></p>',
      onCreate: () => {
        isInitializing = false;
      },
      onTransaction: ({ editor }) => {
        // Reassign the wrapper object to force Svelte 5 reactivity (official pattern)
        editorState = { editor };
      },
      onUpdate: ({ editor }) => {
        if (!isInitializing) {
          isInternalUpdate = true;
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const md = (editor.storage as any).markdown.getMarkdown();
          oninput(md);
        }
      },
      editorProps: {
        attributes: {
          class: 'prose prose-sm max-w-none focus:outline-none min-h-[280px] p-3 text-sm',
        },
      },
    });
  });

  onDestroy(() => {
    editorState.editor?.destroy();
  });

  function toggleBold() {
    editorState.editor?.chain().focus().toggleBold().run();
  }

  function toggleItalic() {
    editorState.editor?.chain().focus().toggleItalic().run();
  }

  function toggleBulletList() {
    editorState.editor?.chain().focus().toggleBulletList().run();
  }

  function toggleOrderedList() {
    editorState.editor?.chain().focus().toggleOrderedList().run();
  }

  function undo() {
    editorState.editor?.chain().focus().undo().run();
  }

  function redo() {
    editorState.editor?.chain().focus().redo().run();
  }

  function handleSave() {
    if (editorState.editor) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const md = (editorState.editor.storage as any).markdown.getMarkdown();
      onsave(md);
    }
  }

  // Update editor content only when value prop changes externally (e.g. restore default)
  // Skip when the change came from the editor itself (user typing) to avoid feedback loop
  $effect(() => {
    // Only track `value` as dependency — use untrack for editor to avoid re-running on every keystroke
    const currentValue = value;

    if (isInternalUpdate) {
      isInternalUpdate = false;
      return;
    }

    untrack(() => {
      const ed = editorState.editor;
      if (ed && currentValue) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const currentMd = (ed.storage as any).markdown?.getMarkdown() ?? '';
        if (currentValue !== currentMd) {
          ed.commands.setContent(currentValue, { emitUpdate: false });
        }
      }
    });
  });
</script>

<div class="rounded-lg border border-input bg-background">
  {#if editorState.editor}
    <!-- Toolbar -->
    <div class="flex items-center gap-1 border-b border-border bg-muted/20 p-2">
      <Button
        type="button"
        onclick={toggleBold}
        variant={editorState.editor.isActive('bold') ? 'default' : 'ghost'}
        size="icon"
        title="Bold (Ctrl+B)"
      >
        <BoldIcon class="size-4" />
      </Button>

      <Button
        type="button"
        onclick={toggleItalic}
        variant={editorState.editor.isActive('italic') ? 'default' : 'ghost'}
        size="icon"
        title="Italic (Ctrl+I)"
      >
        <ItalicIcon class="size-4" />
      </Button>

      <div class="w-px h-5 bg-border mx-1"></div>

      <Button
        type="button"
        onclick={toggleBulletList}
        variant={editorState.editor.isActive('bulletList') ? 'default' : 'ghost'}
        size="icon"
        title="Bullet List"
      >
        <ListIcon class="size-4" />
      </Button>

      <Button
        type="button"
        onclick={toggleOrderedList}
        variant={editorState.editor.isActive('orderedList') ? 'default' : 'ghost'}
        size="icon"
        title="Ordered List"
      >
        <ListOrdered class="size-4" />
      </Button>

      <div class="w-px h-5 bg-border mx-1"></div>

      <Button
        type="button"
        onclick={undo}
        variant="ghost"
        size="icon"
        disabled={!editorState.editor.can().undo()}
        title="Undo (Ctrl+Z)"
      >
        <Undo2 class="size-4" />
      </Button>

      <Button
        type="button"
        onclick={redo}
        variant="ghost"
        size="icon"
        disabled={!editorState.editor.can().redo()}
        title="Redo (Ctrl+Shift+Z)"
      >
        <Redo2 class="size-4" />
      </Button>

      <div class="flex-1"></div>

      {#if onrestore}
        <Button
          type="button"
          onclick={onrestore}
          variant="ghost"
          size="sm"
          title="Khôi phục prompt mặc định"
        >
          <RotateCcw class="size-4" />
          <span>Restore default</span>
        </Button>
      {/if}

      <Button
        type="button"
        onclick={handleSave}
        variant={isSaved ? 'secondary' : 'default'}
        size="sm"
        title={isSaved ? 'Saved' : 'Save (Ctrl+S)'}
      >
        {#if isSaved}
          <Check class="size-4" />
          <span>Saved</span>
        {:else}
          <Save class="size-4" />
          <span>Save</span>
        {/if}
      </Button>
    </div>
  {/if}

  <!-- Editor - Always render this so it's available for mounting -->
  <div bind:this={element} class="overflow-auto max-h-100"></div>
</div>
