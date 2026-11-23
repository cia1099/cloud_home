import { $, $$ } from './dom.ts';

function pressOnly(group: HTMLElement, pressed: HTMLElement): void {
  for (const b of $$('[data-f]', group)) b.setAttribute('aria-pressed', String(b === pressed));
}

/** /requirements/: filter by status chip and by search text (id and both languages). */
export function initRequirements(): void {
  const list = $('#reqs');
  const filters = $('#reqFilters');
  const search = $<HTMLInputElement>('#reqSearch');
  if (!list || !filters || !search) return;
  let status = 'all';
  let query = '';
  const apply = (): void => {
    let shown = 0;
    for (const li of $$('.req', list)) {
      const ok = (status === 'all' || li.dataset.s === status) && (!query || (li.dataset.q ?? '').includes(query));
      li.classList.toggle('hide', !ok);
      if (ok) shown++;
    }
    for (const area of $$('.area', list)) area.hidden = !$('.req:not(.hide)', area);
    const none = $('#noRes');
    if (none) none.hidden = shown > 0;
  };
  filters.addEventListener('click', (e) => {
    const b = (e.target as Element).closest<HTMLElement>('[data-f]');
    if (!b) return;
    status = b.dataset.f ?? 'all';
    pressOnly(filters, b);
    apply();
  });
  search.addEventListener('input', () => {
    query = search.value.trim().toLowerCase();
    apply();
  });
  list.addEventListener('click', (e) => {
    if (matchMedia('(max-width:860px)').matches && !(e.target as Element).closest('a')) (e.target as Element).closest('.req')?.classList.toggle('open');
  });
}

/** /timeline/: filter events by type; a month heading hides when none of its events is shown. */
export function initTimeline(): void {
  const list = $('#tl');
  const filters = $('#tlFilters');
  if (!list || !filters) return;
  filters.addEventListener('click', (e) => {
    const b = (e.target as Element).closest<HTMLElement>('[data-f]');
    if (!b) return;
    pressOnly(filters, b);
    const type = b.dataset.f ?? 'all';
    let heading: HTMLElement | null = null;
    let any = false;
    const flush = (): void => {
      if (heading) heading.hidden = !any;
    };
    for (const li of $$('li', list)) {
      if (li.classList.contains('tl-month')) {
        flush();
        heading = li;
        any = false;
        continue;
      }
      const ok = type === 'all' || li.dataset.t === type;
      li.hidden = !ok;
      any ||= ok;
    }
    flush();
  });
}
