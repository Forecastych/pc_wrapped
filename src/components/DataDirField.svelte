<script lang="ts">
  import { untrack } from 'svelte';
  import { pickFolder, setDataDir } from '../lib/api';

  // current — где данные сейчас (null на первом запуске), start — что показать в поле сразу
  let { current, start, applyLabel, onDone }: { current: string | null; start: string; applyLabel: string; onDone: (path: string) => void } =
    $props();

  let path = $state(untrack(() => start));
  let busy = $state(false);
  let error = $state('');

  async function pick() {
    error = '';
    const p = await pickFolder(path);
    if (p) path = p;
  }

  async function apply() {
    busy = true;
    error = '';
    try {
      onDone(await setDataDir(path));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="dir">
  <code title={path}><bdi dir="ltr">{path}</bdi></code>
  <button class="change" onclick={pick} disabled={busy}>Изменить…</button>
</div>
{#if path !== current}
  <button class="apply" onclick={apply} disabled={busy}>{busy ? 'Переносим…' : applyLabel}</button>
{/if}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
  .dir {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
  }
  code {
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
    border-radius: 10px;
    border: 2px solid color-mix(in srgb, var(--ink) 25%, transparent);
    background: white;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl; /* если не влезает — обрезаем начало пути, а не имя папки */
    text-align: left;
  }
  .change {
    flex: none;
    background: none;
    border: 2px solid color-mix(in srgb, var(--ink) 30%, transparent);
    border-radius: 999px;
    padding: 7px 16px;
  }
  .apply {
    margin-top: 12px;
    border: 0;
    background: var(--violet);
    color: var(--milk);
    font-weight: 700;
    padding: 10px 22px;
    border-radius: 999px 999px 999px 6px;
  }
  .error {
    margin: 12px 0 0;
    padding: 10px 14px;
    background: #ffd9d9;
    color: #8a1020;
    border-radius: 12px;
    font-size: 14px;
    font-weight: 600;
  }
</style>
