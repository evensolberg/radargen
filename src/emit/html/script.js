// radargen's interactive layer (FR-48, FR-56, FR-98; architecture §5).
//
// Constant: it holds nothing from the CSV and reads what it needs from the
// page, so every page carries these same bytes (NFR-4) and no CSV text is
// ever script (NFR-6). It requests nothing (NFR-5). Without it the page
// still works: markers keep their native outline, and the controls it
// drives stay hidden.
(() => {
  'use strict';

  // Every attribute, class and id this script uses. A Rust test checks
  // each value against the constant the page is written with.
  const N = {
    marker: 'data-marker',
    legend: 'data-legend-row',
    markerIndicator: 'data-marker-indicator',
    focused: 'data-focused',
    narrowQuery: 'data-narrow-query',
    theme: 'data-theme',
    js: 'js',
    toggle: 'theme-toggle',
    skip: 'skip',
    arrangement: 'arrangement',
  };

  const root = document.documentElement;
  const main = document.querySelector('main');
  // The control last focused, as its key and kind, never as an element: a
  // breakpoint change hides the element, and Chromium then moves focus to
  // the body, so only the key survives (architecture §5).
  const state = { focus: null };

  const shown = () =>
    [...document.querySelectorAll('.' + N.arrangement)].find(
      (a) => getComputedStyle(a).display !== 'none',
    );
  const control = (node) =>
    node instanceof Element ? node.closest('[' + N.marker + '],[' + N.legend + ']') : null;
  const kindOf = (el) => (el.hasAttribute(N.marker) ? N.marker : N.legend);
  const keyOf = (el) => el.getAttribute(kindOf(el));

  // A marker's focus ring lives in the indicator layer, after every marker,
  // so it is shown by attribute rather than by a rule per key (rg-jri). The
  // focused indicator moves last in its layer, so no later indicator shown
  // for hover or selection covers it (FR-48, D-66).
  const ring = (el, on) => {
    const indicator = el
      .closest('svg')
      .querySelector('[' + N.markerIndicator + '="' + keyOf(el) + '"]');
    if (!indicator) return;
    indicator.toggleAttribute(N.focused, on);
    if (on) indicator.parentNode.append(indicator);
  };
  const showRing = (el) => {
    if (kindOf(el) === N.marker) ring(el, el.matches(':focus-visible'));
  };

  document.addEventListener('focusin', (event) => {
    const el = control(event.target);
    state.focus = el ? { key: keyOf(el), kind: kindOf(el) } : null;
    if (el) showRing(el);
  });
  document.addEventListener('focusout', (event) => {
    const el = control(event.target);
    if (el && kindOf(el) === N.marker) ring(el, false);
  });
  // A key press after a mouse focus makes the focus visible.
  document.addEventListener('keydown', () => {
    const el = control(document.activeElement);
    if (el) showRing(el);
  });

  // A breakpoint change hides the arrangement holding the focus; focus the
  // same item in the one now shown. When the change event fires, Chromium
  // may not yet have moved focus off the hidden element, so focus counts
  // as lost on the body or anywhere in an arrangement no longer shown.
  matchMedia(main.getAttribute(N.narrowQuery)).addEventListener('change', () => {
    const now = document.activeElement;
    const where = now?.closest('.' + N.arrangement);
    const lost = !now || now === document.body || (where && where !== shown());
    if (!state.focus || !lost) return;
    const { key, kind } = state.focus;
    const el = shown()?.querySelector('[' + kind + '="' + key + '"]');
    if (el) el.focus();
  });

  // FR-56: dark while pressed; until pressed, the viewer's preference.
  const toggle = document.querySelector('.' + N.toggle);
  const prefersDark = matchMedia('(prefers-color-scheme: dark)');
  const dark = () => {
    const chosen = root.getAttribute(N.theme);
    return chosen ? chosen === 'dark' : prefersDark.matches;
  };
  const pressed = () => toggle.setAttribute('aria-pressed', String(dark()));
  toggle.addEventListener('click', () => {
    root.setAttribute(N.theme, dark() ? 'light' : 'dark');
    pressed();
  });
  prefersDark.addEventListener('change', pressed);

  // FR-98: past the markers to the legend of the arrangement shown.
  const skip = document.querySelector('.' + N.skip);
  skip.addEventListener('click', () => {
    shown()?.querySelector('[' + N.legend + ']')?.focus();
  });

  root.classList.add(N.js);
  toggle.hidden = false;
  skip.hidden = false;
  pressed();
})();
