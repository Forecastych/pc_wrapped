/** Svelte-экшен: уменьшает шрифт, пока текст не влезет по ширине (без переносов внутри слова). */
export function fit(node: HTMLElement, opts: { max: number; min?: number; key?: unknown }) {
  let cfg = opts;
  const run = () => {
    const min = cfg.min ?? 16;
    let size = cfg.max;
    node.style.fontSize = `${size}px`;
    while (size > min && (node.scrollWidth > node.clientWidth + 1 || node.scrollHeight > (node.dataset.maxh ? +node.dataset.maxh : Infinity))) {
      size -= 2;
      node.style.fontSize = `${size}px`;
    }
  };
  run();
  document.fonts?.ready.then(run);
  return {
    update(next: { max: number; min?: number; key?: unknown }) {
      cfg = next;
      run();
    },
  };
}
