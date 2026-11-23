import type { BespokeCtx } from '../../src/build/render/report.ts';

/** A bespoke milestone page: returns <main id="main">; the shared header, footer, tokens and components still apply. */
export default function page({ report, asset, imageSize, kit, site }: BespokeCtx): string {
  const { T, plainBi, ic } = kit;
  const { w, h } = imageSize('media/hero.svg');
  const done = site.counts.done;
  return `<div class="ms"><main id="main">
  <section class="ms-hero"><div class="wrap">
    <span class="badge-ms">${ic('tag')}<span>${T('Milestone', '里程碑')}</span></span>
    <h1>${plainBi(report.title)}</h1>
    <p class="lede">${plainBi(report.summary)}</p>
    <div class="device"><div class="dev-bar"><i></i><i></i><i></i></div><img src="${asset('media/hero.svg')}" width="${w}" height="${h}" alt=""></div>
  </div></section>
  <nav class="ms-nav" aria-label="On this page"><div class="wrap"><a href="#numbers">${T('By the numbers', '数字')}</a></div></nav>
  <section class="ms-sec wrap" id="numbers"><p class="eyebrow">${T('By the numbers', '用数字说话')}</p>
    <div class="bignums"><div class="io"><b data-count="${done}">${done}</b><span>${T('requirements done', '项需求已完成')}</span></div></div>
    ${kit.pager()}
  </section>
</main></div>`;
}
