<script lang="ts">
  import { onMount } from 'svelte';
  import { getSettings, getToday, onLive, onSettings, startDragging, toggleMini } from '../lib/api';
  import { metricDef } from '../lib/metrics';
  import type { DayStats, Settings } from '../lib/types';

  let day = $state<DayStats | null>(null);
  let settings = $state<Settings | null>(null);

  const totals = $derived(
    day
      ? {
          keys: day.keys,
          clicks: day.clicksLeft + day.clicksRight + day.clicksMiddle + day.clicksOther,
          mouseMm: day.mouseMm,
          wheelNotches: day.wheelNotches,
          activeSeconds: day.activeSeconds,
        }
      : null,
  );
  // В мини-окне места мало: 12 345 → 12,3k
  function short(m: { key: string; value: (t: any) => string }, t: any) {
    const n = m.key === 'keys' ? t.keys : m.key === 'clicks' ? t.clicks : null;
    if (n !== null && n >= 10000) return `${(Math.floor(n / 100) / 10).toString().replace('.', ',')}k`;
    return m.value(t);
  }
  const shown = $derived((settings?.miniMetrics ?? []).slice(0, 4).map(metricDef));

  onMount(() => {
    let offs: (() => void)[] = [];
    (async () => {
      settings = await getSettings();
      day = await getToday();
      offs.push(await onLive((d) => (day = d)));
      offs.push(await onSettings((s) => (settings = s)));
    })();
    return () => offs.forEach((f) => f());
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="mini" onmousedown={(e) => e.button === 0 && !(e.target as HTMLElement).closest('button') && startDragging()}>
  <button class="close" onclick={() => toggleMini()} aria-label="Скрыть мини-окно">×</button>
  {#if totals}
    <div class="grid" class:one={shown.length === 1}>
      {#each shown as m (m.key)}
        <div class="cell">
          <span class="dot" style:background={m.color}></span>
          <b>{short(m, totals)}<small>{m.unit(totals)}</small></b>
          <span class="t">{m.title}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .mini {
    position: relative;
    width: 100vw;
    height: 100vh;
    background: color-mix(in srgb, var(--ink) 92%, transparent);
    border-radius: 18px;
    padding: 12px 14px;
    user-select: none;
    cursor: grab;
    overflow: hidden;
  }
  .close {
    position: absolute;
    top: 4px;
    right: 6px;
    border: 0;
    background: none;
    font-size: 18px;
    line-height: 1;
    opacity: 0;
    padding: 4px;
    transition: opacity 0.15s;
  }
  .mini:hover .close {
    opacity: 0.7;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 12px;
    height: 100%;
    align-content: center;
  }
  .grid.one {
    grid-template-columns: 1fr;
  }
  .cell {
    display: grid;
    grid-template-columns: 8px 1fr;
    column-gap: 7px;
    align-items: baseline;
    min-width: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    align-self: center;
  }
  b {
    font-family: var(--display);
    font-weight: 400;
    font-size: 19px;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-variant-numeric: tabular-nums;
  }
  b small {
    font-size: 12px;
    margin-left: 3px;
  }
  .t {
    grid-column: 2;
    font-size: 12px;
    opacity: 0.7;
  }
</style>
