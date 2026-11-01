<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getAutostart, inTauri, openDataDir, setAutostart, setSettings } from '../lib/api';
  import { METRICS } from '../lib/metrics';
  import type { Metric, Settings } from '../lib/types';

  let { settings, onClose }: { settings: Settings; onClose: (next?: Settings) => void } = $props();

  let draft = $state<Settings>(untrack(() => structuredClone($state.snapshot(settings))));
  let autostart = $state(false);
  let saving = $state(false);

  onMount(async () => {
    autostart = await getAutostart();
  });

  function toggle(list: Metric[], m: Metric, max: number): Metric[] {
    if (list.includes(m)) return list.filter((x) => x !== m);
    return list.length >= max ? list : [...list, m];
  }

  async function save() {
    saving = true;
    try {
      await setSettings(draft);
      await setAutostart(autostart);
      onClose(structuredClone($state.snapshot(draft)));
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onClose()} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <h2 id="settings-title">Настройки</h2>

    <fieldset>
      <legend>Значки в трее</legend>
      <p class="hint">Каждый счётчик — отдельный значок с цифрой рядом с часами. В Windows 11 новые значки прячутся под стрелку «^»: перетащите их на панель.</p>
      <div class="chips">
        {#each METRICS as m}
          <label class="chip" style:--c={m.color}>
            <input type="checkbox" checked={draft.trayMetrics.includes(m.key)} onchange={() => (draft.trayMetrics = toggle(draft.trayMetrics, m.key, 5))} />
            {m.title}
          </label>
        {/each}
      </div>
    </fieldset>

    <fieldset>
      <legend>Мини-окно</legend>
      <p class="hint">Маленькое окно поверх всех окон. До четырёх счётчиков. В играх видно только в оконном режиме без рамки.</p>
      <div class="chips">
        {#each METRICS as m}
          <label class="chip" style:--c={m.color}>
            <input type="checkbox" checked={draft.miniMetrics.includes(m.key)} onchange={() => (draft.miniMetrics = toggle(draft.miniMetrics, m.key, 4))} />
            {m.title}
          </label>
        {/each}
      </div>
      <label class="row">
        <input type="checkbox" bind:checked={draft.miniClickthrough} />
        Пропускать клики сквозь мини-окно (двигать его тогда нельзя; выключается в меню трея)
      </label>
    </fieldset>

    <fieldset>
      <legend>Мышка</legend>
      <label class="row">
        DPI мышки
        <input class="num" type="number" min="100" max="32000" step="50" bind:value={draft.dpi} />
      </label>
      <p class="hint">Нужно, чтобы перевести движения сенсора в километры. Обычно 800–1600, точное значение есть в программе мышки или на коробке.</p>
    </fieldset>

    <fieldset>
      <legend>Запуск</legend>
      <label class="row">
        <input type="checkbox" bind:checked={autostart} disabled={!inTauri} />
        Запускать вместе с Windows (тихо, в трей)
      </label>
    </fieldset>

    <p class="privacy">
      Всё хранится только на этом компьютере. Что вы печатали, программа не знает и не записывает: только сколько раз нажата
      каждая клавиша.
      {#if inTauri}<button class="link" onclick={() => openDataDir()}>Открыть папку с данными</button>{/if}
    </p>

    <div class="buttons">
      <button class="ghost" onclick={() => onClose()}>Отмена</button>
      <button class="cta" onclick={save} disabled={saving}>{saving ? 'Сохраняем…' : 'Сохранить'}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(20 10 50 / 0.55);
    display: grid;
    place-items: center;
    padding: 20px;
    z-index: 10;
  }
  .sheet {
    background: var(--milk);
    color: var(--ink);
    border-radius: 28px 28px 28px 8px;
    padding: 24px 26px;
    width: min(560px, 100%);
    max-height: calc(100vh - 40px);
    overflow: auto;
  }
  h2 {
    font-family: var(--display);
    font-weight: 400;
    font-size: 28px;
    margin: 0 0 8px;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 18px 0 0;
  }
  legend {
    font-weight: 700;
    font-size: 16px;
    padding: 0;
    margin-bottom: 4px;
  }
  .hint {
    font-size: 13px;
    opacity: 0.75;
    margin: 0 0 10px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px 6px 8px;
    border-radius: 999px;
    border: 2px solid var(--c);
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
  }
  .chip:has(input:checked) {
    background: var(--c);
  }
  .chip input {
    accent-color: var(--ink);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    margin-top: 10px;
  }
  .num {
    width: 100px;
    padding: 6px 10px;
    border-radius: 10px;
    border: 2px solid color-mix(in srgb, var(--ink) 25%, transparent);
    background: white;
    color: var(--ink);
  }
  .privacy {
    margin: 20px 0 0;
    padding: 12px 14px;
    background: var(--lemon);
    border-radius: 16px 16px 16px 4px;
    font-size: 14px;
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    text-decoration: underline;
    font-weight: 600;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
  .ghost {
    background: none;
    border: 2px solid color-mix(in srgb, var(--ink) 30%, transparent);
    border-radius: 999px;
    padding: 8px 18px;
  }
  .cta {
    border: 0;
    background: var(--violet);
    color: var(--milk);
    font-weight: 700;
    padding: 10px 22px;
    border-radius: 999px 999px 999px 6px;
  }
</style>
