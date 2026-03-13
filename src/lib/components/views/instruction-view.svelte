<script lang="ts">
  import TiptapEditor from '$lib/components/tiptap-editor.svelte';
  import Card from '$lib/components/ui/card.svelte';
  import { DEFAULT_FORM } from '$lib/constants';
  import {
    activeJob,
    saveTranslationInstruction,
    updateForm
  } from '$lib/stores/app-store';

  let isPromptSaved = $state(true);

  function handlePromptInput(html: string): void {
    updateForm('translationInstruction', html);
    isPromptSaved = false;
  }

  function handlePromptSave(html: string): void {
    updateForm('translationInstruction', html);
    saveTranslationInstruction(html);
    isPromptSaved = true;
  }

  function handlePromptRestore(): void {
    const defaultPrompt = DEFAULT_FORM.translationInstruction;
    updateForm('translationInstruction', defaultPrompt);
    saveTranslationInstruction(defaultPrompt);
    isPromptSaved = true;
  }
</script>

<div class="view-enter">
  <Card class="border bg-card">
    <div class="p-4">
      <h2 class="text-base font-semibold mb-3">Prompt cho AI</h2>
      
      <TiptapEditor
        value={$activeJob.form.translationInstruction}
        oninput={handlePromptInput}
        onsave={handlePromptSave}
        onrestore={handlePromptRestore}
        isSaved={isPromptSaved}
      />
    </div>
  </Card>
</div>
