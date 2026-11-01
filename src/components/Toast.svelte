<script lang="ts">
  import { onMount } from 'svelte';
  import { closeWindow, inTauri, onToast, onToastHover } from '../lib/api';
  import type { ToastData } from '../lib/types';

  const SHOW_MS = 4500;
  const ICON: Record<ToastData['kind'], string> = { week: '📅', month: '🗓️', break: '🧘', test: '👋' };

  // Первую карточку Rust кладёт прямо в окно при создании, следующие присылает событием
  const DEMO: ToastData = { kind: 'week', title: 'Твоя неделя за компом', body: '31 ч 20 мин за компом · 184 310 нажатий · 6,4 км мышкой', action: 'lastweek' };
  let toast = $state<ToastData | null>((window as unknown as { __TOAST__?: ToastData }).__TOAST__ ?? (inTauri ? null : DEMO));
  let shown = $state(false);
  // курсор над карточкой: она бледнеет, чтобы было видно, что под ней, — клики и так проходят насквозь
  let ghost = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  function arm(ms = SHOW_MS) {
    clearTimeout(hideTimer);
    hideTimer = setTimeout(hide, ms);
  }

  function hide() {
    shown = false;
    closeTimer = setTimeout(() => closeWindow(), 300);
  }

  onMount(() => {
    requestAnimationFrame(() => (shown = true));
    arm();
    const offs: (() => void)[] = [];
    onToast((t) => {
      toast = t;
      clearTimeout(closeTimer);
      shown = true;
      arm();
    }).then((f) => offs.push(f));
    onToastHover((over) => (ghost = over)).then((f) => offs.push(f));
    return () => offs.forEach((f) => f());
  });
</script>

{#if toast}
  <div class="toast {toast.kind}" class:shown class:ghost role="status">
    <span class="icon" aria-hidden="true">{ICON[toast.kind]}</span>
    <span class="text">
      <b>{toast.title}</b>
      <span class="body">{toast.body}</span>
      {#if toast.action}<span class="more">Щёлкни по значку в трее — покажу итоги</span>{/if}
    </span>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    inset: 6px;
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 10px 14px 10px 12px;
    border-radius: 18px 18px 18px 6px;
    background: var(--ink);
    color: var(--milk);
    text-align: left;
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.35);
    transform: translateX(110%);
    opacity: 0;
    transition:
      transform 0.28s cubic-bezier(0.2, 0.9, 0.3, 1.15),
      opacity 0.2s;
  }
  .toast.shown {
    transform: none;
    opacity: 1;
  }
  .toast.shown.ghost {
    opacity: 0.15;
  }
  .toast::before {
    content: '';
    position: absolute;
    left: 0;
    top: 14px;
    bottom: 14px;
    width: 4px;
    border-radius: 0 4px 4px 0;
    background: var(--accent, var(--violet));
  }
  .week,
  .month {
    --accent: var(--lemon);
  }
  .break {
    --accent: var(--mint);
  }
  .icon {
    flex: none;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent, var(--violet)) 22%, transparent);
    font-size: 22px;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.3;
  }
  b {
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    font-size: 12.5px;
    opacity: 0.85;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    margin-top: 2px;
    font-size: 12.5px;
    font-weight: 700;
    color: var(--accent, var(--violet));
  }
</style>
