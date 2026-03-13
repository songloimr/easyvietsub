<script lang="ts">
  import '../app.css';
  import AppShell from '$lib/components/app-shell.svelte';
  import { Toaster } from 'svelte-sonner';
</script>

<svelte:head>
  <title>EasyVietsub</title>
  <meta
    name="description"
    content="Ứng dụng desktop dịch video/audio sang tiếng Việt, xuất SRT và render hard subtitle."
  />
</svelte:head>

<AppShell />
<Toaster position="bottom-right" richColors />
