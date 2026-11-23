import { $ } from './dom.ts';

/** Phone "More" sheet. */
export function initSheet(): void {
  const sheet = $('#sheet');
  const more = $('#moreBtn');
  if (!sheet || !more) return;
  const close = (): void => {
    sheet.classList.remove('open');
    more.focus();
  };
  more.addEventListener('click', () => {
    sheet.classList.add('open');
    $('.item', sheet)?.focus();
  });
  sheet.addEventListener('click', (e) => {
    if (e.target === sheet) close();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && sheet.classList.contains('open')) close();
  });
}
