<script lang="ts">
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import Button from '$lib/components/ui/button.svelte';
  import { history, setView } from '$lib/stores/app-store';

  let {
    confirmRetranslateOpen = $bindable(false),
    confirmRetranslateOnlyOpen = $bindable(false),
    confirmDeleteModelOpen = $bindable(false),
    confirmDeleteModelId,
    confirmDeleteRuntimeOpen = $bindable(false),
    confirmClearHistoryOpen = $bindable(false),
    errorDialogOpen = $bindable(false),
    errorDialogTitle,
    errorDialogMessage,
    errorDialogDetail,
    successDialogOpen = $bindable(false),
    successDialogTitle,
    successDialogMessage,
    onConfirmRetranslate,
    onConfirmRetranslateOnly,
    onConfirmDeleteModel,
    onConfirmDeleteRuntime,
    onConfirmClearHistory
  }: {
    confirmRetranslateOpen: boolean;
    confirmRetranslateOnlyOpen: boolean;
    confirmDeleteModelOpen: boolean;
    confirmDeleteModelId: string;
    confirmDeleteRuntimeOpen: boolean;
    confirmClearHistoryOpen: boolean;
    errorDialogOpen: boolean;
    errorDialogTitle: string;
    errorDialogMessage: string;
    errorDialogDetail: string;
    successDialogOpen: boolean;
    successDialogTitle: string;
    successDialogMessage: string;
    onConfirmRetranslate: () => void;
    onConfirmRetranslateOnly: () => void;
    onConfirmDeleteModel: () => void;
    onConfirmDeleteRuntime: () => void;
    onConfirmClearHistory: () => void;
  } = $props();
</script>

<!-- Re-translate confirmation dialog -->
<AlertDialog.Root bind:open={confirmRetranslateOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Xác nhận chạy lại</AlertDialog.Title>
      <AlertDialog.Description>
        Chạy lại sẽ xóa toàn bộ kết quả hiện tại. Bạn có muốn tiếp tục?
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>
        <Button variant="outline" size="sm">Hủy</Button>
      </AlertDialog.Cancel>
      <AlertDialog.Action>
        <Button variant="destructive" size="sm" onclick={onConfirmRetranslate}>Tiếp tục</Button>
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<!-- Re-translate only (Gemini only) confirmation dialog -->
<AlertDialog.Root bind:open={confirmRetranslateOnlyOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Dịch lại với Gemini</AlertDialog.Title>
      <AlertDialog.Description>
        Dịch lại sẽ dùng SRT gốc (Whisper) hiện tại và gọi Gemini để dịch sang tiếng Việt. Kết quả dịch cũ (nếu có) sẽ bị thay thế.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>
        <Button variant="outline" size="sm">Hủy</Button>
      </AlertDialog.Cancel>
      <AlertDialog.Action>
        <Button size="sm" onclick={onConfirmRetranslateOnly}>Dịch lại</Button>
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<!-- Delete whisper model confirmation dialog -->
<AlertDialog.Root bind:open={confirmDeleteModelOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Xóa Whisper model</AlertDialog.Title>
      <AlertDialog.Description>
        Xóa Whisper model <strong>{confirmDeleteModelId}</strong> khỏi local cache?
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>
        <Button variant="outline" size="sm">Hủy</Button>
      </AlertDialog.Cancel>
      <AlertDialog.Action>
        <Button variant="destructive" size="sm" onclick={onConfirmDeleteModel}>Xóa</Button>
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<!-- Delete FFmpeg runtime confirmation dialog -->
<AlertDialog.Root bind:open={confirmDeleteRuntimeOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Xóa FFmpeg local</AlertDialog.Title>
      <AlertDialog.Description>
        Xóa FFmpeg local đã cài khỏi app? Bạn có thể cài lại bất kỳ lúc nào.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>
        <Button variant="outline" size="sm">Hủy</Button>
      </AlertDialog.Cancel>
      <AlertDialog.Action>
        <Button variant="destructive" size="sm" onclick={onConfirmDeleteRuntime}>Xóa</Button>
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<!-- Clear history confirmation dialog -->
<AlertDialog.Root bind:open={confirmClearHistoryOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Xóa tất cả history</AlertDialog.Title>
      <AlertDialog.Description>
        Xóa tất cả {$history.length} job khỏi history? Hành động này không thể hoàn tác.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>
        <Button variant="outline" size="sm">Hủy</Button>
      </AlertDialog.Cancel>
      <AlertDialog.Action>
        <Button variant="destructive" size="sm" onclick={onConfirmClearHistory}>Xóa tất cả</Button>
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<!-- Error dialog -->
<Dialog.Root bind:open={errorDialogOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title class="text-destructive">{errorDialogTitle}</Dialog.Title>
      <Dialog.Description>{errorDialogMessage}</Dialog.Description>
    </Dialog.Header>
    {#if errorDialogDetail}
      <pre class="mt-2 max-h-64 overflow-auto whitespace-pre-wrap wrap-break-word rounded-md border bg-muted/30 px-3 py-2 text-xs">{errorDialogDetail}</pre>
    {/if}
    <Dialog.Footer>
      <Dialog.Close>
        <Button variant="outline" size="sm" onclick={() => { errorDialogOpen = false; setView('log'); }}>Xem logs</Button>
      </Dialog.Close>
      <Dialog.Close>
        <Button size="sm" onclick={() => { errorDialogOpen = false; }}>Đóng</Button>
      </Dialog.Close>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Success dialog -->
<Dialog.Root bind:open={successDialogOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title class="text-green-600 dark:text-green-400">{successDialogTitle}</Dialog.Title>
      <Dialog.Description>{successDialogMessage}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Dialog.Close>
        <Button size="sm" onclick={() => { successDialogOpen = false; }}>Đóng</Button>
      </Dialog.Close>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
