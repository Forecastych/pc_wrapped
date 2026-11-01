<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { toPng } from 'html-to-image';
  import Slide from './Slide.svelte';
  import { buildSlides } from '../lib/slides';
  import type { Summary } from '../lib/aggregate';
  import type { PeriodKey } from '../lib/types';

  let { summary, period, onClose }: { summary: Summary; period: PeriodKey; onClose: () => void } = $props();
  const onBack = () => onClose();

  // Снимок на момент открытия: живые цифры не должны перескакивать посреди просмотра
  const stats = untrack(() => $state.snapshot(summary)) as Summary;
  const slides = buildSlides(stats);
  let i = $state(0);
  let stageW = $state(0);
  let stageH = $state(0);
  let still = $state(false);
  let saving = $state(false);
  let toast = $state<string | null>(null);
  let cardEl = $state<HTMLDivElement>();

  const scale = $derived(stageW && stageH ? Math.min(stageW / 360, stageH / 654, 1.5) : 1);
  const canShare = typeof navigator !== 'undefined' && 'canShare' in navigator;

  function go(d: number) {
    i = Math.max(0, Math.min(slides.length - 1, i + d));
  }

  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.closest?.('select, input')) return;
    if (e.key === 'ArrowRight' || e.key === ' ') {
      e.preventDefault();
      go(1);
    } else if (e.key === 'ArrowLeft') go(-1);
    else if (e.key === 'Escape') onBack();
  }

  let downX = 0;
  function pointerDown(e: PointerEvent) {
    downX = e.clientX;
  }
  function pointerUp(e: PointerEvent) {
    const dx = e.clientX - downX;
    if (Math.abs(dx) > 40) return go(dx < 0 ? 1 : -1);
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    go(e.clientX - r.left < r.width * 0.35 ? -1 : 1);
  }

  async function render(): Promise<Blob> {
    const node = cardEl?.querySelector('.card') as HTMLElement;
    still = true;
    await tick();
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    try {
      const url = await toPng(node, { pixelRatio: 3, cacheBust: false });
      return await (await fetch(url)).blob();
    } finally {
      still = false;
    }
  }

  function flash(t: string) {
    toast = t;
    setTimeout(() => (toast = null), 2200);
  }

  async function save() {
    if (saving) return;
    saving = true;
    try {
      const blob = await render();
      const name = `pc-wrapped-${period}-${new Date().toISOString().slice(0, 10)}-${i + 1}.png`;
      const file = new File([blob], name, { type: 'image/png' });
      if (canShare && matchMedia('(pointer: coarse)').matches && navigator.canShare({ files: [file] })) {
        await navigator.share({ files: [file] }).catch(() => {});
      } else {
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = name;
        document.body.appendChild(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(a.href), 4000);
        flash('Картинка сохранена');
      }
    } catch {
      flash('Не получилось сохранить картинку');
    } finally {
      saving = false;
    }
  }

</script>

<svelte:window onkeydown={onKey} />

<div class="story">
  <div class="top">
    <button class="ghost" onclick={onBack}>← К статистике</button>
  </div>

  <div class="stage" bind:clientWidth={stageW} bind:clientHeight={stageH}>
    <div class="scaled" style:width="{360 * scale}px" style:height="{(640 + 14) * scale}px">
      <div class="inner" style:transform="scale({scale})">
        <div class="progress" aria-hidden="true">
          {#each slides as _, k}<span class:done={k <= i}></span>{/each}
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="tapzone" class:still bind:this={cardEl} onpointerdown={pointerDown} onpointerup={pointerUp}>
          {#key i}
            <Slide s={stats} {period} def={slides[i]} />
          {/key}
        </div>
      </div>
    </div>
  </div>

  <div class="bottom">
    <button class="nav" onclick={() => go(-1)} disabled={i === 0} aria-label="Назад">‹</button>
    <button class="save" onclick={save} disabled={saving}>{saving ? 'Рисуем…' : 'Сохранить картинку'}</button>
    <button class="nav" onclick={() => go(1)} disabled={i === slides.length - 1} aria-label="Дальше">›</button>
  </div>
  <p class="count" aria-live="polite">{i + 1} из {slides.length}</p>

  {#if toast}<div class="toast" role="status">{toast}</div>{/if}
</div>

<style>
  .story {
    position: fixed;
    inset: 0;
    z-index: 20;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    background: var(--violet-deep);
    padding: 12px 16px calc(10px + env(safe-area-inset-bottom));
    overflow: hidden;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    max-width: 560px;
    width: 100%;
    margin: 0 auto 10px;
  }
  .ghost {
    background: none;
    border: 0;
    padding: 8px 0;
    opacity: 0.9;
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }
  .scaled {
    position: relative;
  }
  .inner {
    width: 360px;
    transform-origin: top left;
    position: absolute;
    top: 0;
    left: 0;
  }
  .progress {
    display: flex;
    gap: 3px;
    height: 4px;
    margin-bottom: 10px;
  }
  .progress span {
    flex: 1;
    border-radius: 2px;
    background: color-mix(in srgb, var(--milk) 28%, transparent);
  }
  .progress span.done {
    background: var(--milk);
  }
  .tapzone {
    border-radius: 26px;
    overflow: hidden;
    cursor: pointer;
    user-select: none;
    touch-action: pan-y;
    box-shadow: 0 18px 50px rgb(20 8 60 / 0.45);
  }
  .tapzone :global(.b) {
    animation: pop 0.4s cubic-bezier(0.2, 1.4, 0.4, 1) backwards;
  }
  .tapzone :global(.b:nth-child(2)) {
    animation-delay: 0.25s;
  }
  .tapzone :global(.b:nth-child(3)) {
    animation-delay: 0.5s;
  }
  .tapzone :global(.b:nth-child(n + 4)) {
    animation-delay: 0.7s;
  }
  .tapzone.still :global(*) {
    animation: none !important;
  }
  .tapzone.still :global([data-noexport]) {
    visibility: hidden;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.94);
    }
  }
  .bottom {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 14px;
    margin-top: 12px;
  }
  .nav {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--milk) 45%, transparent);
    background: transparent;
    font-size: 26px;
    line-height: 1;
    padding-bottom: 3px;
  }
  .nav:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .save {
    border: 0;
    background: var(--lemon);
    color: var(--ink);
    font-weight: 700;
    font-size: 16px;
    padding: 13px 24px;
    border-radius: 999px 999px 999px 8px;
  }
  .save:disabled {
    cursor: progress;
    opacity: 0.85;
  }
  .count {
    text-align: center;
    font-size: 13px;
    opacity: 0.65;
    margin: 8px 0 0;
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: 90px;
    transform: translateX(-50%);
    background: var(--ink);
    padding: 10px 18px;
    border-radius: 999px;
    font-size: 15px;
  }
</style>
