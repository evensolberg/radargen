// radargen's interactive layer (FR-37, FR-38, FR-48, FR-49, FR-56, FR-72,
// FR-73, FR-98; architecture §5).
//
// Constant: it holds nothing from the CSV and reads what it needs from the
// page, so every page carries these same bytes (NFR-4) and no CSV text is
// ever script (NFR-6). It requests nothing (NFR-5). Without it the page
// still works: markers keep their native outline, the table shows every
// row, and the controls it drives stay hidden.
(() => {
  'use strict';

  // Every attribute, class and id this script uses. A Rust test checks
  // each value against the constant the page is written with.
  const N = {
    row: 'data-row',
    marker: 'data-marker',
    legend: 'data-legend-row',
    hit: 'data-hit',
    markerIndicator: 'data-marker-indicator',
    focused: 'data-focused',
    selected: 'data-selected',
    hovered: 'data-hovered',
    expanded: 'aria-expanded',
    controls: 'aria-controls',
    panel: 'detail',
    heading: 'detail-heading',
    close: 'detail-close',
    empty: 'detail-empty',
    prose: 'prose',
    tableView: 'table-view',
    narrowQuery: 'data-narrow-query',
    theme: 'data-theme',
    toDark: 'Switch to dark theme',
    toLight: 'Switch to light theme',
    js: 'js',
    toggle: 'theme-toggle',
    legendToggle: 'legend-toggle',
    legendState: 'data-legend',
    legendHidden: 'hidden',
    hideLegend: 'Hide legend',
    showLegend: 'Show legend',
    skipLegend: 'Skip to the legend',
    skipTable: 'Skip to the table',
    words: 'visually-hidden',
    skip: 'skip',
    arrangement: 'arrangement',
    legendScroll: 'wide-legend',
  };

  const root = document.documentElement;
  const main = document.querySelector('main');
  // Items are held as their key and kind, never as elements: a breakpoint
  // change hides an element, and Chromium then moves focus to the body, so
  // only the key survives (architecture §5). `focus` is the control last
  // focused, `open` the item whose panel shows and what opened it, and
  // `hovered` the key under the pointer.
  const TABLE = 'table';
  const state = { focus: null, open: null, hovered: null };

  // `[name="key"]`: every element of an item carries its key (§5).
  const by = (name, key) => '[' + name + '="' + key + '"]';
  const shown = () =>
    [...document.querySelectorAll('.' + N.arrangement)].find(
      (a) => getComputedStyle(a).display !== 'none',
    );
  const control = (node) =>
    node instanceof Element ? node.closest('[' + N.marker + '],[' + N.legend + ']') : null;
  const kindOf = (el) => (el.hasAttribute(N.marker) ? N.marker : N.legend);
  // Rendered and visible: a hidden wide legend stays in its track,
  // invisible, so its rows still have boxes (RG-102).
  const isShown = (el) =>
    !!el?.getClientRects().length && getComputedStyle(el).visibility !== 'hidden';
  // Focus is lost when it sits on nothing, on the page itself, or on an
  // element no longer shown: one in an arrangement a breakpoint hid, or a
  // legend row a press hid. WebKit, and Firefox on macOS, do not focus a
  // clicked button, so a press leaves focus on the body or on that row.
  const focusLost = () => {
    const now = document.activeElement;
    return !now || now === document.body || !isShown(now);
  };
  const keyOf = (el) => el.getAttribute(kindOf(el));

  // A marker's focus ring lives in the indicator layer, after every marker,
  // so it is shown by attribute rather than by a rule per key (rg-jri). The
  // focused indicator moves last in its layer, so no later indicator shown
  // for hover or selection covers it (FR-48, D-66).
  const ring = (el, on) => {
    const indicator = el
      .closest('svg')
      .querySelector(by(N.markerIndicator, keyOf(el)));
    if (!indicator) return;
    indicator.toggleAttribute(N.focused, on);
    if (on && indicator !== indicator.parentNode.lastElementChild) indicator.parentNode.append(indicator);
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
    if (!el) return;
    if (kindOf(el) === N.marker) ring(el, false);
    // Still rendered, so the reader left it, and a later breakpoint change
    // must not pull focus back. A blur from the breakpoint itself comes
    // once the element is hidden, which keeps the key.
    if (el.getClientRects().length > 0) state.focus = null;
  });

  // A breakpoint change hides the arrangement holding the focus; focus the
  // same item in the one now shown. When the change event fires, Chromium
  // may not yet have moved focus off the hidden element, so focus counts
  // as lost on the body or anywhere in an arrangement no longer shown.
  matchMedia(main.getAttribute(N.narrowQuery)).addEventListener('change', () => {
    if (!state.focus || !focusLost()) return;
    const { key, kind } = state.focus;
    const el = shown()?.querySelector(by(kind, key));
    if (el) el.focus();
  });

  // FR-38's panel, built from the item's table row (D-80): its heading is
  // the number and name, and each non-empty cell after the name becomes a
  // term and its value, cloned so a link keeps the table's escaped address.
  // The prototypes for those come from the page, so the script makes no
  // markup of its own (NFR-6).
  const panel = document.getElementById(N.panel);
  const heading = panel.querySelector('.' + N.heading);
  const list = panel.querySelector('dl');
  const empty = panel.querySelector('.' + N.empty);
  const termShape = list.querySelector('dt');
  const valueShape = list.querySelector('dd');
  list.replaceChildren();
  const details = document.querySelector('details.' + N.tableView);
  const tableRow = (key) => document.querySelector('tr' + by(N.row, key));
  const tableButton = (key) => tableRow(key)?.querySelector('button');

  const fill = (key) => {
    const row = tableRow(key);
    const titles = [...row.closest('table').querySelectorAll('thead th')];
    const cells = [...row.children];
    const nameAt = cells.findIndex((c) => c.tagName === 'TH');
    const number = cells[0].textContent;
    const name = cells[nameAt].textContent;
    heading.textContent = number ? number + '. ' + name : name;
    list.replaceChildren();
    cells.slice(nameAt + 1).forEach((cell, i) => {
      if (!cell.textContent.trim()) return;
      const term = termShape.cloneNode(false);
      term.textContent = titles[nameAt + 1 + i].textContent;
      const value = valueShape.cloneNode(false);
      for (const child of cell.childNodes) value.append(child.cloneNode(true));
      if (cell.classList.contains(N.prose)) value.classList.add(N.prose);
      list.append(term, value);
    });
    list.hidden = list.childElementCount === 0;
    empty.hidden = !list.hidden;
  };

  // The sticky panel must never hide what has focus (WCAG 2.4.11): not on
  // the page, nor in the wide legend's own scrolling box (D-85). The box is
  // padded by as much of it as the panel covers, so a row focused in it is
  // scrolled within it, above the panel, rather than the page moving and
  // taking the radar with it. Scroll padding adds no room to scroll into,
  // so the box gets that much real padding too, or its last rows could
  // never clear the panel.
  const boxes = [...document.querySelectorAll('.' + N.legendScroll)];
  // Every size is read before any style is written, so a scroll forces
  // no extra layout.
  const pad = () => {
    const top = panel.getBoundingClientRect().top;
    const height = panel.offsetHeight;
    // The overlap only: never more than the box, or padding would grow it.
    const unders = boxes.map((box) => {
      const r = box.getBoundingClientRect();
      return panel.hidden ? 0 : r.bottom - Math.max(top, r.top);
    });
    root.style.scrollPaddingBottom = panel.hidden ? '' : height + 'px';
    boxes.forEach((box, i) => {
      const by = unders[i] > 0 ? unders[i] + 'px' : '';
      box.style.scrollPaddingBottom = by;
      box.style.paddingBottom = by;
    });
  };
  new ResizeObserver(pad).observe(panel);
  addEventListener('scroll', pad, { passive: true });
  addEventListener('resize', pad);

  // Every element of an item that matches `own`: its controls, its table
  // row and button, and its marker's indicators.
  const parts = (key, own) =>
    document.querySelectorAll(own + by(N.row, key) + ',' + by(N.markerIndicator, key));

  // FR-72: selection shows on every element of the item, both arrangements
  // and the table; FR-73's marks come from the page's fixed rules.
  const mark = (key, on) => {
    for (const el of parts(key, '')) {
      el.toggleAttribute(N.selected, on);
      if (el.hasAttribute(N.expanded)) el.setAttribute(N.expanded, String(on));
    }
  };

  // Where focus returns for `key`: its opener in the arrangement shown if
  // still rendered, else its marker there (a legend row hidden since it
  // opened the panel), else its table button, or the table's toggle while
  // the table is closed.
  const opener = (key, kind) => {
    const rendered = (k) => {
      const el = shown()?.querySelector(by(k, key));
      return isShown(el) ? el : null;
    };
    if (kind !== TABLE) {
      const el = rendered(kind) ?? rendered(N.marker);
      if (el) return el;
    }
    return details.open ? tableButton(key) : details.querySelector('summary');
  };

  const open = (key, kind) => {
    if (state.open) mark(state.open.key, false);
    state.open = { key, kind };
    fill(key);
    mark(key, true);
    panel.hidden = false;
    pad();
    heading.focus({ preventScroll: true });
    // Sticky lifts the panel from below the viewport, never down from above
    // it: opened from deep in the table, it would be out of sight.
    const box = panel.getBoundingClientRect();
    if (box.bottom <= 0 || box.top >= innerHeight) panel.scrollIntoView({ block: 'nearest' });
  };
  const close = (returnFocus) => {
    if (!state.open) return;
    const { key, kind } = state.open;
    mark(key, false);
    state.open = null;
    panel.hidden = true;
    pad();
    if (returnFocus) opener(key, kind)?.focus();
  };
  // Activating the open item again closes it, leaving focus where the
  // activation put it (FR-38).
  const activate = (key, kind) => (state.open?.key === key ? close(false) : open(key, kind));

  // The control an event's target stands for: a marker, a legend row, or the
  // marker a hit area enlarges (FR-94).
  const target = (node) => {
    if (!(node instanceof Element)) return null;
    const hit = node.closest('[' + N.hit + ']');
    if (!hit) return control(node);
    return hit.closest('svg').querySelector(by(N.marker, hit.getAttribute(N.hit)));
  };

  document.addEventListener('click', (event) => {
    const node = event.target;
    if (!(node instanceof Element)) return;
    if (node.closest('.' + N.close)) return close(true);
    const el = target(node);
    if (el) {
      // A hit area takes no focus, so its marker does, and is where focus
      // stays if this activation closes the panel.
      if (el !== control(node)) el.focus({ preventScroll: true });
      return activate(keyOf(el), kindOf(el));
    }
    const button = node.closest('tr[' + N.row + '] button');
    if (button) activate(button.getAttribute(N.row), TABLE);
  });

  // FR-48: Enter and Space work as a click, as on a native button; FR-49:
  // Escape closes the panel and returns focus. Escape acts only from the
  // panel or the open item's own control, so it never moves focus from
  // anywhere else.
  const svgButton = (node) => (control(node) === node ? node : null);
  // Space activates on release only where it was pressed, as a native
  // button does.
  let spaceOn = null;
  document.addEventListener('keydown', (event) => {
    const active = document.activeElement;
    const focusedControl = control(active);
    if (focusedControl) showRing(focusedControl);
    if (event.defaultPrevented || event.isComposing) return;
    const el = svgButton(event.target);
    if (el && event.key === 'Enter') {
      event.preventDefault();
      if (!event.repeat) activate(keyOf(el), kindOf(el));
    } else if (el && event.key === ' ') {
      event.preventDefault();
      spaceOn = el;
    } else if (event.key === 'Escape' && state.open) {
      // From the panel, focus goes back to the opener; from one of the open
      // item's own controls, it stays where the reader is.
      if (panel.contains(active)) {
        event.preventDefault();
        close(true);
      } else if (active?.getAttribute(N.expanded) === 'true') {
        event.preventDefault();
        close(false);
      }
    }
  });
  document.addEventListener('keyup', (event) => {
    if (event.key !== ' ') return;
    const el = svgButton(event.target);
    if (el && el === spaceOn) activate(keyOf(el), kindOf(el));
    spaceOn = null;
  });

  // FR-37: the pointer over a marker lights its legend row, and the
  // reverse. A touch has no hover to end, so it sets none.
  const hover = (key) => {
    if (state.hovered === key) return;
    const set = (k, on) => {
      for (const el of parts(k, '[role=button]')) el.toggleAttribute(N.hovered, on);
    };
    if (state.hovered) set(state.hovered, false);
    state.hovered = key;
    if (key) set(key, true);
  };
  document.addEventListener('pointerover', (event) => {
    if (event.pointerType === 'touch') return;
    const el = target(event.target);
    hover(el ? keyOf(el) : null);
  });
  document.addEventListener('pointerout', (event) => {
    if (!event.relatedTarget) hover(null);
  });

  // FR-56: the viewer's preference until a press chooses; the label says
  // what the next press does. A page drawn with --palette has one set of
  // colours and no toggle, so none of this runs there.
  const toggle = document.querySelector('.' + N.toggle);
  if (toggle) {
    const prefersDark = matchMedia('(prefers-color-scheme: dark)');
    const dark = () => {
      const chosen = root.getAttribute(N.theme);
      return chosen ? chosen === 'dark' : prefersDark.matches;
    };
    // The hidden words name the button; the stylesheet picks its icon.
    const words = toggle.querySelector('.' + N.words);
    const label = () => {
      words.textContent = dark() ? N.toLight : N.toDark;
    };
    toggle.addEventListener('click', () => {
      root.setAttribute(N.theme, dark() ? 'light' : 'dark');
      label();
    });
    prefersDark.addEventListener('change', label);
    label();
    toggle.hidden = false;
  }

  // FR-98, RG-102: past the markers to the legend of the arrangement
  // shown, or to the table while the legend is hidden.
  const skip = document.querySelector('.' + N.skip);
  const plotted = !!document.querySelector('[' + N.legend + ']');
  const isLegendHidden = () => root.getAttribute(N.legendState) === N.legendHidden;
  skip.addEventListener('click', () => {
    if (isLegendHidden()) details.querySelector('summary').focus();
    else shown()?.querySelector('[' + N.legend + ']')?.focus();
  });

  // RG-102: the legend button hides and shows both arrangements' legends.
  // Its name says what a press does, and the skip link follows the state.
  const legendToggle = document.querySelector('.' + N.legendToggle);
  const legendWords = legendToggle.querySelector('.' + N.words);
  const showLegendState = () => {
    const hidden = isLegendHidden();
    legendWords.textContent = hidden ? N.showLegend : N.hideLegend;
    skip.textContent = hidden ? N.skipTable : N.skipLegend;
  };
  legendToggle.addEventListener('click', () => {
    if (isLegendHidden()) root.removeAttribute(N.legendState);
    else root.setAttribute(N.legendState, N.legendHidden);
    showLegendState();
    if (focusLost()) legendToggle.focus();
  });

  // D-66: every control says whether its panel is open, and names it.
  for (const el of document.querySelectorAll(
    '[role=button][' + N.row + '],tr[' + N.row + '] button',
  )) {
    el.setAttribute(N.expanded, 'false');
    el.setAttribute(N.controls, N.panel);
  }
  root.classList.add(N.js);
  // With nothing plotted there is no legend to hide or skip to.
  showLegendState();
  legendToggle.hidden = !plotted;
  skip.hidden = !plotted;
})();
