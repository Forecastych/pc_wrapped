<script lang="ts">
  import type { Summary } from '../lib/aggregate';
  import { km, scrollMeters } from '../lib/aggregate';
  import type { SlideDef } from '../lib/slides';
  import type { PeriodKey } from '../lib/types';
  import { count, hours, kmText, num, pct, plural } from '../lib/format';
  import { activeFun, clicksFun, keysFun, mouseFun, scrollFun } from '../lib/fun';
  import { MONTHS, dateLabel, iso, parseIso, periodRange } from '../lib/dates';
  import { keyName, KEYBOARD } from '../lib/keyboard';
  import { fit } from '../lib/fit';

  let { s, def, period }: { s: Summary; def: SlideDef; period: PeriodKey } = $props();

  const lastWeek = periodRange('lastweek', new Date(), '');
  const lastMonth = parseIso(periodRange('lastmonth', new Date(), '').from);
  const PERIOD_TITLE: Record<PeriodKey, string> = {
    today: 'Твой день за компом',
    week: 'Твоя неделя за компом',
    month: 'Твой месяц за компом',
    year: `Твой ${new Date().getFullYear()} за компом`,
    all: 'Всё время за компом',
    lastweek: 'Твоя неделя за компом',
    lastmonth: `Твой ${MONTHS[lastMonth.getMonth()]} за компом`,
  };
  const FOOT: Record<PeriodKey, string> = {
    today: dateLabel(iso(new Date())),
    week: 'последние 7 дней',
    month: 'последние 30 дней',
    year: String(new Date().getFullYear()),
    all: 'за всё время',
    lastweek: `${dateLabel(lastWeek.from)} – ${dateLabel(lastWeek.to)}`,
    lastmonth: `${MONTHS[lastMonth.getMonth()]} ${lastMonth.getFullYear()}`,
  };
  const withDate = $derived(period !== 'today');

  const mouseKm = $derived(km(s.mouseMm));
  const scrollM = $derived(scrollMeters(s.wheelNotches));
  const topKeys = $derived(s.keyCounts.slice(0, 5));
  const maxKey = $derived(Math.max(1, ...Object.values(s.keyMap)));
  const topApps = $derived(s.apps.slice(0, 5));
  const maxApp = $derived(Math.max(1, ...topApps.map((a) => a[1])));
  const maxHour = $derived(Math.max(1, ...s.hourlyActive));
  const peakHour = $derived(s.hourlyActive.indexOf(Math.max(...s.hourlyActive)));
  const owl = $derived(s.nightShare >= 0.08 ? 'Ночная сова' : peakHour < 13 ? 'Жаворонок' : 'Дневной житель');
  const leftShare = $derived(s.clicks ? s.clicksLeft / s.clicks : 0);
  const backspace = $derived(s.keyMap[8] ?? 0);
  const heat = (v: number) => (v ? 0.2 + 0.8 * Math.sqrt(v / maxKey) : 0.08);
</script>

<div class="card" style:background={def.bg} style:color={def.fg} data-key={def.key}>
  <div class="body">
    {#if def.key === 'intro'}
      <p class="kicker">pc_wrapped</p>
      <h2 class="title" use:fit={{ max: 54, min: 28, key: period }}>{PERIOD_TITLE[period]}</h2>
      <div class="bubbles">
        <div class="b in">Я всё считал: клавиши, клики, километры мышкой</div>
        <div class="b in">Что печатал, не смотрел. Только сколько</div>
        <div class="b out">ну давай, показывай</div>
      </div>
      <p class="tap" data-noexport>Нажимайте справа, чтобы листать</p>
    {:else if def.key === 'active'}
      <p class="kicker">За компом</p>
      <div class="huge mid" use:fit={{ max: 72, min: 30, key: s.activeSeconds }}>{hours(s.activeSeconds)}</div>
      <p class="label">{activeFun(s.activeSeconds)}</p>
      <div class="bubbles">
        {#if withDate}<div class="b in">За компом {count(s.dayCount, 'день', 'дня', 'дней')}, в среднем {hours(s.activeSeconds / Math.max(1, s.dayCount))} в день</div>{/if}
        <div class="b out">Простой дольше двух минут не считаю</div>
      </div>
    {:else if def.key === 'mouse'}
      <p class="kicker">Мышка проехала по столу</p>
      <div class="huge" use:fit={{ max: 112, min: 40, key: mouseKm }}>{kmText(mouseKm)}</div>
      <p class="label">{plural(Math.round(mouseKm), 'километр', 'километра', 'километров')}</p>
      <div class="bubbles">
        <div class="b in">{mouseFun(s.mouseMm / 1000)[0].toUpperCase() + mouseFun(s.mouseMm / 1000).slice(1)}</div>
        {#if withDate && s.bestMouseDay}
          <div class="b out">Рекорд: {kmText(km(s.bestMouseDay.mm))} км за {dateLabel(s.bestMouseDay.date)}</div>
        {/if}
      </div>
    {:else if def.key === 'scroll'}
      <p class="kicker">Колёсиком прокручено</p>
      <div class="huge" use:fit={{ max: 112, min: 40, key: scrollM }}>{num(scrollM)}</div>
      <p class="label">{plural(Math.round(scrollM), 'метр', 'метра', 'метров')} ленты</p>
      <div class="bubbles">
        <div class="b in">{scrollFun(scrollM)[0].toUpperCase() + scrollFun(scrollM).slice(1)}</div>
        <div class="b out">{count(s.wheelNotches, 'щелчок', 'щелчка', 'щелчков')} колеса</div>
      </div>
    {:else if def.key === 'clicks'}
      <p class="kicker">Кликов</p>
      <div class="huge" use:fit={{ max: 112, min: 40, key: s.clicks }}>{num(s.clicks)}</div>
      <p class="label">{clicksFun(s.clicks)}</p>
      <div class="split">
        <div class="half" style:flex-grow={Math.max(s.clicksLeft, 1)}><span>{pct(leftShare)}</span><small>левой</small></div>
        <span class="gap"></span>
        <div class="half other" style:flex-grow={Math.max(s.clicks - s.clicksLeft, 1)}><span>{pct(1 - leftShare)}</span><small>остальными</small></div>
      </div>
    {:else if def.key === 'keys'}
      <p class="kicker">Нажатий на клавиши</p>
      <div class="huge" use:fit={{ max: 112, min: 40, key: s.keys }}>{num(s.keys)}</div>
      <p class="label">{keysFun(s.keys)}</p>
      <div class="bubbles">
        {#if s.peakKpm}
          <div class="b in">Самая бешеная минута: {count(s.peakKpm.value, 'нажатие', 'нажатия', 'нажатий')}{withDate ? `, ${dateLabel(s.peakKpm.date)}` : ''} в {s.peakKpm.at}</div>
        {/if}
        {#if backspace}
          <div class="b out">Backspace: {count(backspace, 'раз', 'раза', 'раз')}. Это {pct(backspace / Math.max(1, s.keys))} всех нажатий</div>
        {/if}
      </div>
    {:else if def.key === 'topkeys'}
      <p class="kicker">Любимые клавиши</p>
      <h2 class="name-big" use:fit={{ max: 60, min: 26, key: topKeys[0][0] }}>{keyName(topKeys[0][0])}</h2>
      <p class="label">{count(topKeys[0][1], 'нажатие', 'нажатия', 'нажатий')}</p>
      <div class="kb">
        {#each KEYBOARD.slice(1) as row}
          <div class="kb-row">
            {#each row as key}
              <span class="key" style:flex-grow={key.w ?? 1} style:opacity={heat(s.keyMap[key.vk] ?? 0)}></span>
            {/each}
          </div>
        {/each}
      </div>
      <div class="times">
        {#each topKeys.slice(1, 5) as [vk, n]}
          <div class="time-row"><span>{keyName(vk)}</span><b>{num(n)}</b></div>
        {/each}
      </div>
    {:else if def.key === 'apps'}
      <p class="kicker">Куда уходит время</p>
      <h2 class="name-big" use:fit={{ max: 44, min: 22, key: topApps[0][0] }}>{topApps[0][0]}</h2>
      <p class="label">{hours(topApps[0][1])}, {pct(topApps[0][1] / Math.max(1, s.activeSeconds))} всего времени</p>
      <ol class="rank">
        {#each topApps.slice(1) as [name, sec]}
          <li>
            <div class="rank-row"><span class="rank-name">{name}</span><span>{hours(sec)}</span></div>
            <div class="bar" style:width="{(sec / maxApp) * 100}%"></div>
          </li>
        {/each}
      </ol>
    {:else if def.key === 'clock'}
      <p class="kicker">Твой режим</p>
      <h2 class="name-big" use:fit={{ max: 52, min: 26, key: owl }}>{owl}</h2>
      <p class="label">чаще всего за компом в {String(peakHour).padStart(2, '0')}:00</p>
      <div class="hours" role="img" aria-label="Активность по часам">
        {#each s.hourlyActive as v, h}
          <div class="h-col"><div class="h-bar" class:night={h < 6} style:height="{(v / maxHour) * 100}%"></div></div>
        {/each}
      </div>
      <div class="h-axis"><span>0</span><span>6</span><span>12</span><span>18</span><span>24</span></div>
      <div class="bubbles">
        {#if s.typicalStart}<div class="b in">Обычно садишься в {s.typicalStart}, встаёшь в {s.typicalEnd}</div>{/if}
        <div class="b out">{pct(s.nightShare)} времени с полуночи до шести</div>
      </div>
    {:else if def.key === 'records'}
      <p class="kicker">Рекорды</p>
      <div class="noms">
        {#if s.longestSession}
          <div class="nom">
            <span class="nom-t">Самая длинная сессия</span>
            <b class="nom-n">{hours(s.longestSession.sec)}</b>
            <span class="nom-v">{withDate ? dateLabel(s.longestSession.date) : 'без перерыва дольше пяти минут'}</span>
          </div>
        {/if}
        {#if s.peakKpm}
          <div class="nom">
            <span class="nom-t">Самая бешеная минута</span>
            <b class="nom-n">{num(s.peakKpm.value)} нажатий</b>
            <span class="nom-v">{withDate ? `${dateLabel(s.peakKpm.date)}, ` : ''}{s.peakKpm.at}</span>
          </div>
        {/if}
        {#if withDate && s.bestDay}
          <div class="nom">
            <span class="nom-t">Самый печатный день</span>
            <b class="nom-n">{num(s.bestDay.keys)}</b>
            <span class="nom-v">{dateLabel(s.bestDay.date, period === 'all')}</span>
          </div>
        {/if}
      </div>
    {:else if def.key === 'summary'}
      <p class="kicker">Итоги</p>
      <h2 class="title small" use:fit={{ max: 34, min: 20, key: period }}>{PERIOD_TITLE[period]}</h2>
      <dl class="sum">
        <div><dt>за компом</dt><dd class="txt" use:fit={{ max: 19, min: 12, key: s.activeSeconds }}>{hours(s.activeSeconds)}</dd></div>
        <div><dt>нажатий</dt><dd use:fit={{ max: 24, min: 12, key: s.keys }}>{num(s.keys)}</dd></div>
        <div><dt>мышкой</dt><dd use:fit={{ max: 24, min: 12, key: mouseKm }}>{kmText(mouseKm)} км</dd></div>
        <div><dt>колесом</dt><dd use:fit={{ max: 24, min: 12, key: scrollM }}>{num(scrollM)} м</dd></div>
        <div><dt>кликов</dt><dd use:fit={{ max: 24, min: 12, key: s.clicks }}>{num(s.clicks)}</dd></div>
        {#if topKeys.length}<div><dt>клавиша</dt><dd class="txt" use:fit={{ max: 19, min: 12, key: topKeys[0][0] }}>{keyName(topKeys[0][0])}</dd></div>{/if}
        {#if topApps.length}<div class="wide"><dt>программа</dt><dd class="txt" use:fit={{ max: 19, min: 12, key: topApps[0][0] }}>{topApps[0][0]}</dd></div>{/if}
      </dl>
      <div class="bubbles">
        <div class="b in">Покажи друзьям: у кого мышка набегала больше?</div>
        <div class="b out">сейчас сравним</div>
      </div>
    {/if}
  </div>
  <div class="foot">
    <span>{FOOT[period]}</span>
    <span>pc_wrapped</span>
  </div>
</div>

<style>
  .card {
    width: 360px;
    height: 640px;
    display: flex;
    flex-direction: column;
    padding: 34px 26px 18px;
    overflow: hidden;
    font-family: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .kicker {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 14px;
    opacity: 0.85;
  }
  .title {
    font-family: var(--display);
    font-weight: 400;
    font-size: 54px;
    line-height: 0.98;
    margin: 4px 0 26px;
  }
  .title.small {
    font-size: 34px;
    margin-bottom: 18px;
  }
  .huge {
    font-family: var(--display);
    font-size: 112px;
    line-height: 0.95;
    letter-spacing: -0.03em;
    margin: 6px 0 4px -3px;
    white-space: nowrap;
  }
  .huge.mid {
    white-space: normal;
  }
  .label {
    font-size: 18px;
    margin: 0 0 18px;
    line-height: 1.3;
  }
  .name-big {
    font-family: var(--display);
    font-weight: 400;
    font-size: 52px;
    line-height: 1.02;
    margin: 4px 0 8px;
  }
  .tap {
    margin: auto 0 0;
    font-size: 14px;
    opacity: 0.7;
  }
  .bubbles {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: auto;
  }
  .b {
    max-width: 88%;
    padding: 9px 14px;
    font-size: 16px;
    line-height: 1.3;
    border-radius: 18px;
    background: var(--milk);
    color: var(--ink);
  }
  .b.in {
    align-self: flex-start;
    border-bottom-left-radius: 5px;
  }
  .b.out {
    align-self: flex-end;
    border-bottom-right-radius: 5px;
    background: var(--ink);
    color: var(--milk);
  }
  .card[data-key='intro'] .b.out,
  .card[data-key='clock'] .b.out {
    background: var(--lemon);
    color: var(--ink);
  }

  .split {
    display: flex;
    height: 190px;
    margin-top: auto;
  }
  .half {
    flex-basis: 0;
    min-width: 92px;
    background: var(--ink);
    color: var(--milk);
    border-radius: 22px 22px 22px 6px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 14px;
  }
  .half.other {
    background: var(--milk);
    color: var(--ink);
    border-radius: 22px 22px 6px 22px;
    align-items: flex-end;
  }
  .half span {
    font-family: var(--display);
    font-size: 30px;
    line-height: 1;
  }
  .half small {
    font-size: 13px;
    margin-top: 4px;
  }
  .gap {
    width: 6px;
  }

  .kb {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 6px 0 16px;
  }
  .kb-row {
    display: flex;
    gap: 3px;
  }
  .key {
    flex-basis: 0;
    height: 18px;
    border-radius: 4px;
    background: var(--lemon);
  }
  .times {
    display: flex;
    flex-direction: column;
    gap: 9px;
    margin-top: auto;
  }
  .time-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    font-size: 17px;
    border-bottom: 2px solid color-mix(in srgb, currentColor 22%, transparent);
    padding-bottom: 7px;
  }
  .time-row b {
    font-family: var(--display);
    font-weight: 400;
    font-size: 20px;
  }

  .rank {
    list-style: none;
    margin: auto 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 13px;
  }
  .rank-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 5px;
  }
  .rank-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 22px;
    background: var(--ink);
    border-radius: 6px 14px 14px 6px;
    min-width: 8px;
  }

  .hours {
    display: grid;
    grid-template-columns: repeat(24, minmax(0, 1fr));
    gap: 3px;
    height: 150px;
    align-items: end;
    margin-top: 8px;
  }
  .h-col {
    height: 100%;
    display: flex;
    align-items: flex-end;
  }
  .h-bar {
    width: 100%;
    min-height: 2px;
    background: var(--lemon);
    border-radius: 3px 3px 1px 1px;
  }
  .h-bar.night {
    background: var(--sky);
  }
  .h-axis {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    opacity: 0.7;
    margin-top: 4px;
  }

  .noms {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .nom {
    display: flex;
    flex-direction: column;
    background: var(--milk);
    color: var(--ink);
    border-radius: 18px 18px 18px 5px;
    padding: 12px 16px 13px;
  }
  .nom-t {
    font-size: 13px;
    font-weight: 700;
    color: var(--violet);
  }
  .nom-n {
    font-family: var(--display);
    font-weight: 400;
    font-size: 28px;
    line-height: 1.15;
  }
  .nom-v {
    font-size: 14px;
  }

  .sum {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin: 0;
  }
  .sum div {
    background: var(--ink);
    color: var(--milk);
    border-radius: 16px 16px 16px 5px;
    padding: 10px 14px 12px;
    min-width: 0;
  }
  .sum .wide {
    grid-column: span 2;
  }
  .sum dt {
    font-size: 13px;
    opacity: 0.75;
  }
  .sum dd {
    margin: 2px 0 0;
    font-family: var(--display);
    font-size: 24px;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
  }
  .sum dd.txt {
    font-size: 19px;
    padding-top: 4px;
  }

  .foot {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    opacity: 0.7;
    padding-top: 12px;
  }
</style>
