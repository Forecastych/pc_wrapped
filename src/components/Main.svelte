<script lang="ts">
  import { onMount } from 'svelte';
  import { firstDate, getDays, getSettings, inTauri, onLive, onOpenStory, saveExport, takePendingStory, toggleMini } from '../lib/api';
  import { aggregate, type Summary } from '../lib/aggregate';
  import { PERIOD_LABEL, dateLabel, iso, periodRange } from '../lib/dates';
  import { BASELINE_HINT, baselineRange, baselineTotals, delta, deltaText } from '../lib/compare';
  import { daysCsv } from '../lib/csv';
  import { count, hours, kmText, num, pct } from '../lib/format';
  import { KEYBOARD, keyName } from '../lib/keyboard';
  import { METRICS } from '../lib/metrics';
  import { fit } from '../lib/fit';
  import type { DayStats, PeriodKey, Settings as SettingsT } from '../lib/types';
  import SettingsPanel from './Settings.svelte';
  import Story from './Story.svelte';

  const PERIODS: PeriodKey[] = ['today', 'week', 'month', 'year', 'all'];
  let period = $state<PeriodKey>('today');
  let days = $state<DayStats[]>([]);
  let first = $state<string>('');
  let settings = $state<SettingsT | null>(null);
  let showSettings = $state(false);
  let story = $state<{ summary: Summary; period: PeriodKey } | null>(null);
  let loading = $state(true);
  let baseline = $state<{ period: PeriodKey; days: DayStats[] } | null>(null);
  let note = $state('');

  const s: Summary = $derived(aggregate(days));
  const totals = $derived({ keys: s.keys, clicks: s.clicks, mouseMm: s.mouseMm, wheelNotches: s.wheelNotches, activeSeconds: s.activeSeconds });
  const maxHour = $derived(Math.max(1, ...s.hourlyActive));
  const maxKey = $derived(Math.max(1, ...Object.values(s.keyMap)));
  const topApps = $derived(s.apps.slice(0, 7));
  const maxApp = $derived(Math.max(1, ...topApps.map((a) => a[1])));
  const backspace = $derived(s.keyMap[8] ?? 0);
  // для «сегодня» база зависит от часа, поэтому пересчитывается вместе с живыми цифрами
  const base = $derived.by(() => {
    void s.activeSeconds; // зависимость от живых цифр: новый тик — новый час для сравнения
    return baseline?.period === period ? baselineTotals(period, baseline.days, new Date()) : null;
  });

  async function load(p: PeriodKey) {
    loading = true;
    const now = new Date();
    const r = periodRange(p, now, first || '2000-01-01');
    const b = baselineRange(p, now, first);
    const [cur, prev] = await Promise.all([getDays(r.from, r.to), b ? getDays(b.from, b.to) : null]);
    if (p !== period) return; // пока грузили, переключили период
    days = cur;
    baseline = prev && { period: p, days: prev };
    loading = false;
  }

  /** Итоги слайдами. Прошлую неделю или месяц (из уведомления) грузим отдельно, текущий экран не трогаем. */
  async function openStory(p: PeriodKey) {
    if (p === period) {
      story = { summary: s, period: p };
      return;
    }
    const r = periodRange(p, new Date(), first || '2000-01-01');
    const d = await getDays(r.from, r.to);
    if (d.length) story = { summary: aggregate(d), period: p };
  }

  function flashNote(t: string) {
    note = t;
    setTimeout(() => (note = ''), 4000);
  }

  async function exportCsv() {
    const name = `pc_wrapped_${period}_${iso(new Date())}.csv`;
    try {
      const path = await saveExport(name, new TextEncoder().encode(daysCsv(days)));
      flashNote(`Таблица сохранена: ${path}`);
    } catch (e) {
      flashNote(`Не получилось сохранить таблицу: ${e}`);
    }
  }

  const STORY_ACTIONS: string[] = ['lastweek', 'lastmonth'];
  function storyFromToast(action: string | null) {
    if (!action || !STORY_ACTIONS.includes(action)) return;
    showSettings = false;
    openStory(action as PeriodKey);
  }

  function pick(p: PeriodKey) {
    period = p;
    load(p);
  }

  onMount(() => {
    let off: (() => void) | null = null;
    let offStory: (() => void) | null = null;
    (async () => {
      first = await firstDate();
      settings = await getSettings();
      await load(period);
      offStory = await onOpenStory(storyFromToast);
      storyFromToast(await takePendingStory());
      // Живое обновление: подменяем сегодняшний день в текущем наборе
      off = await onLive((d) => {
        const i = days.findIndex((x) => x.date === d.date);
        if (i >= 0) days[i] = d;
        else if (period === 'today' || days.length === 0 || d.date > days[days.length - 1].date) {
          if (period === 'today') days = [d];
          else days = [...days, d];
        }
      });
    })();
    return () => {
      off?.();
      offStory?.();
    };
  });

  const heat = (v: number) => (v ? 0.18 + 0.82 * Math.sqrt(v / maxKey) : 0.06);
</script>

<div class="app">
  <header>
    <span class="mark">pc_wrapped</span>
    <nav aria-label="Период">
      {#each PERIODS as p}
        <button class:on={period === p} onclick={() => pick(p)} aria-pressed={period === p}>{PERIOD_LABEL[p]}</button>
      {/each}
    </nav>
    <div class="actions">
      <button class="ghost" onclick={() => toggleMini()}>Мини-окно</button>
      <button class="ghost" onclick={() => (showSettings = true)}>Настройки</button>
      <button class="ghost" onclick={exportCsv} disabled={!s.dayCount} title="Таблица по дням за выбранный период, открывается в Excel">CSV</button>
      <button class="cta" onclick={() => openStory(period)} disabled={!s.dayCount}>Итоги слайдами</button>
    </div>
  </header>

  {#if !inTauri}
    <p class="demo-note">Демо-режим: это выдуманные данные. В приложении здесь будет ваша статистика.</p>
  {/if}

  <main class:loading>
    <section class="tiles" aria-label="Главные счётчики">
      {#each METRICS as m (m.key)}
        <div class="tile" style:--c={m.color}>
          <span class="t-head">
            <span class="t-title">{m.title}</span>
            {#if base}
              {@const d = delta(m.raw(totals), m.raw(base))}
              {#if d !== null}<span class="t-delta" title="{deltaText(d)} {BASELINE_HINT[period]}">{deltaText(d)}</span>{/if}
            {/if}
          </span>
          <span class="t-value" use:fit={{ max: 34, min: 16, key: m.value(totals) }}>{m.value(totals)}{#if m.unit(totals)}<small>{m.unit(totals)}</small>{/if}</span>
          <span class="t-fun">{m.fun(totals)}</span>
        </div>
      {/each}
    </section>
    {#if base && BASELINE_HINT[period]}<p class="base-hint">Стрелки на плитках — больше или меньше, {BASELINE_HINT[period]}.</p>{/if}

    {#if s.dayCount === 0 && !loading}
      <div class="empty">
        <h2>Пока пусто</h2>
        <p>Программа уже считает. Поработайте за компьютером, и через минуту здесь появятся цифры. Счётчики за сегодня видны и в трее.</p>
      </div>
    {:else}
      <div class="row">
        <section class="panel hours">
          <h2>Когда вы за компом</h2>
          <div class="bars" role="img" aria-label="Активность по часам">
            {#each s.hourlyActive as v, h}
              <div class="bar-col">
                <div class="bar" style:height="{(v / maxHour) * 100}%" class:night={h < 6}></div>
                {#if h % 3 === 0}<span>{h}</span>{/if}
              </div>
            {/each}
          </div>
          <p class="facts">
            {#if s.typicalStart}Обычно садитесь в <b>{s.typicalStart}</b>, встаёте в <b>{s.typicalEnd}</b>. {/if}
            Ночью, с полуночи до шести: <b>{pct(s.nightShare)}</b> времени.
          </p>
        </section>

        <section class="panel apps">
          <h2>Программы</h2>
          {#if topApps.length}
            <ol>
              {#each topApps as [name, sec]}
                <li>
                  <div class="app-row"><span class="app-name">{name}</span><span>{hours(sec)}</span></div>
                  <div class="app-bar" style:width="{(sec / maxApp) * 100}%"></div>
                </li>
              {/each}
            </ol>
          {:else}
            <p class="muted">Программы появятся, когда наберётся активное время.</p>
          {/if}
        </section>
      </div>

      <section class="panel kb-panel">
        <div class="kb-head">
          <h2>Клавиатура</h2>
          <p class="muted">
            Топ: {s.keyCounts.slice(0, 3).map(([vk, n]) => `${keyName(vk)} ${num(n)}`).join(', ') || 'пока ничего'}.
            {#if backspace}Backspace нажат {count(backspace, 'раз', 'раза', 'раз')}.{/if}
          </p>
        </div>
        <div class="kb" role="img" aria-label="Тепловая карта клавиатуры">
          {#each KEYBOARD as row}
            <div class="kb-row">
              {#each row as key}
                <div class="key" class:cold={!s.keyMap[key.vk]} style:flex-grow={key.w ?? 1} style:--a={heat(s.keyMap[key.vk] ?? 0)} title="{key.label}: {num(s.keyMap[key.vk] ?? 0)}">
                  {key.label}
                </div>
              {/each}
            </div>
          {/each}
        </div>
      </section>

      <section class="records" aria-label="Рекорды">
        {#if s.peakKpm}
          <div class="rec">
            <span>Самая бешеная минута</span>
            <b>{num(s.peakKpm.value)} нажатий</b>
            <small>{period === 'today' ? `в ${s.peakKpm.at}` : `${dateLabel(s.peakKpm.date)}, ${s.peakKpm.at}`}</small>
          </div>
        {/if}
        {#if s.longestSession}
          <div class="rec">
            <span>Самая длинная сессия</span>
            <b>{hours(s.longestSession.sec)}</b>
            <small>{period === 'today' ? 'без перерыва дольше пяти минут' : dateLabel(s.longestSession.date)}</small>
          </div>
        {/if}
        {#if s.bestDay && period !== 'today'}
          <div class="rec">
            <span>Самый печатный день</span>
            <b>{num(s.bestDay.keys)}</b>
            <small>{dateLabel(s.bestDay.date, period === 'all')}</small>
          </div>
        {/if}
        {#if s.bestMouseDay && period !== 'today'}
          <div class="rec">
            <span>Мышка набегала больше всего</span>
            <b>{kmText(s.bestMouseDay.mm / 1e6)} км</b>
            <small>{dateLabel(s.bestMouseDay.date, period === 'all')}</small>
          </div>
        {/if}
        {#if period !== 'today'}
          <div class="rec">
            <span>Дней за компом</span>
            <b>{num(s.dayCount)}</b>
            <small>в среднем {hours(s.activeSeconds / Math.max(1, s.dayCount))} в день</small>
          </div>
        {/if}
      </section>
    {/if}
  </main>
</div>

{#if showSettings && settings}
  <SettingsPanel {settings} onClose={(next) => { if (next) settings = next; showSettings = false; }} />
{/if}

{#if story}
  <Story summary={story.summary} period={story.period} onClose={() => (story = null)} />
{/if}

{#if note}<div class="note" role="status">{note}</div>{/if}

<style>
  .app {
    min-height: 100vh;
    padding: 18px 28px 40px;
    max-width: 1280px;
    margin: 0 auto;
  }
  header {
    display: flex;
    align-items: center;
    gap: 20px;
    flex-wrap: wrap;
  }
  .mark {
    font-family: var(--display);
    font-size: 22px;
    color: var(--lemon);
    margin-right: 6px;
  }
  nav {
    display: flex;
    gap: 4px;
    background: var(--violet-deep);
    padding: 4px;
    border-radius: 999px;
  }
  nav button {
    border: 0;
    background: none;
    padding: 7px 14px;
    border-radius: 999px;
    font-weight: 600;
    font-size: 15px;
  }
  nav button.on {
    background: var(--milk);
    color: var(--ink);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .ghost {
    background: none;
    border: 2px solid color-mix(in srgb, var(--milk) 45%, transparent);
    border-radius: 999px;
    padding: 7px 16px;
    font-size: 15px;
  }
  .ghost:hover {
    border-color: var(--milk);
  }
  .cta {
    border: 0;
    background: var(--lemon);
    color: var(--ink);
    font-weight: 700;
    padding: 9px 20px;
    border-radius: 999px 999px 999px 6px;
    box-shadow: 0 4px 0 var(--ink);
    font-size: 15px;
  }
  .cta:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .demo-note {
    margin: 14px 0 0;
    font-size: 14px;
    opacity: 0.8;
  }
  main {
    transition: opacity 0.15s;
  }
  main.loading {
    opacity: 0.6;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 12px;
    margin: 22px 0 18px;
  }
  .tile {
    background: var(--c);
    color: var(--ink);
    border-radius: 22px 22px 22px 6px;
    padding: 14px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .t-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .t-title {
    font-weight: 700;
    font-size: 14px;
  }
  .t-delta {
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    padding: 1px 7px;
    border-radius: 999px;
    background: rgb(29 20 66 / 0.12);
    cursor: help;
  }
  .base-hint {
    margin: -8px 0 16px;
    font-size: 13px;
    opacity: 0.75;
  }
  .note {
    position: fixed;
    left: 50%;
    bottom: 22px;
    transform: translateX(-50%);
    max-width: min(720px, calc(100vw - 40px));
    padding: 10px 18px;
    border-radius: 14px;
    background: var(--ink);
    color: var(--milk);
    font-size: 14px;
    box-shadow: 0 6px 20px rgb(0 0 0 / 0.3);
    z-index: 30;
    overflow-wrap: anywhere;
  }
  .t-value {
    font-family: var(--display);
    font-size: 34px;
    line-height: 1.15;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .t-value small {
    font-size: 0.55em;
    margin-left: 4px;
  }
  .t-fun {
    font-size: 13px;
    line-height: 1.3;
    opacity: 0.85;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr);
    gap: 14px;
  }
  .panel {
    background: var(--ink);
    border-radius: 24px;
    padding: 18px 20px 20px;
    min-width: 0;
  }
  h2 {
    font-family: var(--display);
    font-weight: 400;
    font-size: 20px;
    margin: 0 0 14px;
  }
  .muted {
    opacity: 0.7;
    font-size: 14px;
    margin: 0;
  }
  .bars {
    display: grid;
    grid-template-columns: repeat(24, minmax(0, 1fr));
    gap: 4px;
    height: 150px;
    align-items: end;
  }
  .bar-col {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    opacity: 0.95;
  }
  .bar-col span {
    opacity: 0.6;
    height: 14px;
  }
  .bar-col:not(:has(span))::after {
    content: '';
    height: 14px;
  }
  .bar {
    width: 100%;
    min-height: 2px;
    background: var(--tangerine);
    border-radius: 4px 4px 2px 2px;
  }
  .bar.night {
    background: var(--sky);
  }
  .facts {
    margin: 14px 0 0;
    font-size: 14px;
    opacity: 0.9;
  }
  .apps ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .app-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 14px;
    margin-bottom: 4px;
  }
  .app-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .app-bar {
    height: 8px;
    background: var(--mint);
    border-radius: 4px;
    min-width: 4px;
  }

  .kb-panel {
    margin-top: 14px;
  }
  .kb-head {
    display: flex;
    align-items: baseline;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .kb-head h2 {
    margin: 0;
  }
  .kb {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .kb-row {
    display: flex;
    gap: 5px;
  }
  .key {
    flex-basis: 0;
    height: 38px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    font-size: 12px;
    font-weight: 600;
    background: color-mix(in srgb, var(--lemon) calc(var(--a) * 100%), color-mix(in srgb, var(--milk) 6%, var(--ink)));
    color: var(--ink);
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .key.cold {
    color: color-mix(in srgb, var(--milk) 45%, transparent);
  }

  .records {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 12px;
    margin-top: 14px;
  }
  .rec {
    border: 2px solid color-mix(in srgb, var(--milk) 30%, transparent);
    border-radius: 20px 20px 20px 6px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
  }
  .rec span {
    font-size: 13px;
    opacity: 0.8;
  }
  .rec b {
    font-family: var(--display);
    font-weight: 400;
    font-size: 22px;
    line-height: 1.25;
  }
  .rec small {
    font-size: 13px;
    opacity: 0.75;
  }
  .empty {
    background: var(--ink);
    border-radius: 24px;
    padding: 24px;
  }
  .empty p {
    max-width: 40em;
    margin: 0;
    opacity: 0.85;
  }

  @media (max-width: 1000px) {
    .tiles {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
    .row {
      grid-template-columns: 1fr;
    }
  }
</style>
