<script lang="ts">
  import DataDirField from './DataDirField.svelte';
  import type { DataDir } from '../lib/types';

  // Первый запуск: пока папку не выбрали, данные копятся в %APPDATA% и переедут при выборе.
  // Закрыть окно без выбора нельзя — для этого оно и есть.
  let { dir, onDone }: { dir: DataDir; onDone: (path: string) => void } = $props();
</script>

<div class="backdrop">
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="setup-title">
    <h2 id="setup-title">Где хранить статистику?</h2>
    <p>
      По файлу на день, несколько килобайт каждый. Туда же лягут сохранённые картинки и таблицы
      (подпапка <code>exports</code>).
    </p>
    <p class="hint">
      {#if dir.systemOnly}
        Других жёстких дисков не нашлось, поэтому предлагаем системный. Можно выбрать любую папку.
      {:else}
        Предлагаем несистемный диск, чтобы не засорять C:. Можно выбрать любую папку, сменить её потом можно в настройках.
      {/if}
    </p>
    <DataDirField current={null} start={dir.suggested} applyLabel="Хранить здесь" {onDone} />
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
    z-index: 20;
  }
  .sheet {
    background: var(--milk);
    color: var(--ink);
    border-radius: 28px 28px 28px 8px;
    padding: 24px 26px;
    width: min(520px, 100%);
  }
  h2 {
    font-family: var(--display);
    font-weight: 400;
    font-size: 28px;
    margin: 0 0 8px;
  }
  p {
    font-size: 14px;
    margin: 0 0 8px;
  }
  .hint {
    opacity: 0.75;
  }
</style>
