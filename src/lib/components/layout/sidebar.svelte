<script lang="ts">
  import { VIEW_LABELS } from '$lib/constants';
  import { currentView, setView } from '$lib/stores/app-store';
  import type { ViewKey } from '$lib/types';
  import FileText from '@lucide/svelte/icons/file-text';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import Subtitles from '@lucide/svelte/icons/subtitles';
  import Video from '@lucide/svelte/icons/video';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';

  type NavItem = {
    key: ViewKey;
    icon: typeof FileText;
    label: string;
  };

  const mainNavItems: NavItem[] = [
    { key: 'translate', icon: WandSparkles, label: VIEW_LABELS.translate },
    { key: 'instruction', icon: FileText, label: VIEW_LABELS.instruction },
    { key: 'render', icon: Video, label: VIEW_LABELS.render },
    { key: 'history', icon: HistoryIcon, label: VIEW_LABELS.history },
    { key: 'log', icon: FileText, label: VIEW_LABELS.log }
  ];

  const bottomNavItems: NavItem[] = [
    { key: 'settings', icon: SettingsIcon, label: VIEW_LABELS.settings }
  ];
</script>

<aside
  class="sidebar fixed left-0 top-0 z-40 flex h-screen w-48 flex-col border-r bg-card shadow-lg"
>
  <!-- Logo -->
  <div class="flex h-13 items-center gap-2.5 border-b px-3.5 overflow-hidden">
    <div class="shrink-0 rounded-md bg-primary/10 p-1.5 text-primary">
      <Subtitles class="size-4" />
    </div>
    <span class="sidebar-label whitespace-nowrap text-sm font-semibold">
      EasyVietsub
    </span>
  </div>

  <!-- Main nav items -->
  <nav class="flex-1 space-y-0.5 px-2 py-2 overflow-hidden">
    {#each mainNavItems as item}
      {@const Icon = item.icon}
      {@const isActive = $currentView === item.key}
      <button
        type="button"
        onclick={() => setView(item.key)}
        class="sidebar-item flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors duration-150
          {isActive
            ? 'bg-primary/10 text-primary font-medium'
            : 'text-muted-foreground hover:bg-accent hover:text-foreground'}"
        title={item.label}
      >
        <Icon class="size-4 shrink-0" />
        <span class="sidebar-label whitespace-nowrap">
          {item.label}
        </span>
      </button>
    {/each}
  </nav>

  <!-- Bottom nav (Settings) -->
  <div class="border-t px-2 py-2 overflow-hidden">
    {#each bottomNavItems as item}
      {@const Icon = item.icon}
      {@const isActive = $currentView === item.key}
      <button
        type="button"
        onclick={() => setView(item.key)}
        class="sidebar-item flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors duration-150
          {isActive
            ? 'bg-primary/10 text-primary font-medium'
            : 'text-muted-foreground hover:bg-accent hover:text-foreground'}"
        title={item.label}
      >
        <Icon class="size-4 shrink-0" />
        <span class="sidebar-label whitespace-nowrap">
          {item.label}
        </span>
      </button>
    {/each}
  </div>
</aside>
