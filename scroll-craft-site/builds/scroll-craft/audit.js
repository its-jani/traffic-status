/* ============================================================================
   audit.js · the page's signature move
   ----------------------------------------------------------------------------
   The peak act draws a spec sheet over its own typography. Every number the
   visitor reads here was computed from the rendered page at this viewport: the
   measure is the paragraph's real width divided by the real advance of a "0" in
   its own face, the line count comes from its box height over its leading, the
   tracking is the computed letter-spacing in em, the lamp diameter is the lens
   box as laid out, and every ratio is WCAG contrast between two colours the
   browser resolved.

   The three lamp ratios are the argument of the act: the widget's own colours
   measured against the housing they are read on, so the states are not claimed
   to be legible, they are shown to be.

   Nothing is authored, and nothing here touches the engine.
   ========================================================================== */

(function () {
  'use strict';

  var root = document.querySelector('[data-audit]');
  if (!root) return;

  var act = root.closest('[data-sc-act]');
  var para = root.querySelector('[data-audit-para]');
  var head = root.querySelector('.states__h');
  var lamp = root.querySelector('.lamp');
  if (!act || !para || !head || !lamp) return;

  // DOM order inside the housing is red, amber, green.
  var lenses = root.querySelectorAll('.lamp--live .lamp__led');

  var out = function (key) {
    return root.querySelector('[data-audit-out="' + key + '"]');
  };

  function set(key, text) {
    var el = out(key);
    if (el) el.textContent = text;
  }

  // ---- colour -------------------------------------------------------------

  function parseColor(value) {
    if (!value) return null;
    value = value.trim();
    var m = value.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (m) {
      var h = m[1];
      if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
      return [
        parseInt(h.slice(0, 2), 16),
        parseInt(h.slice(2, 4), 16),
        parseInt(h.slice(4, 6), 16)
      ];
    }
    m = value.match(/rgba?\(([^)]+)\)/i);
    if (m) {
      var parts = m[1].split(/[,\s/]+/).filter(Boolean).map(parseFloat);
      // A fully transparent surface has no colour to measure against.
      if (parts.length >= 3 && (parts.length < 4 || parts[3] > 0)) {
        return [parts[0], parts[1], parts[2]];
      }
    }
    return null;
  }

  function hex(rgb) {
    return '#' + rgb.map(function (v) {
      var s = Math.round(Math.max(0, Math.min(255, v))).toString(16);
      return s.length === 1 ? '0' + s : s;
    }).join('');
  }

  function luminance(rgb) {
    var c = rgb.map(function (v) {
      v /= 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  }

  function ratio(a, b) {
    var la = luminance(a) + 0.05;
    var lb = luminance(b) + 0.05;
    return la > lb ? la / lb : lb / la;
  }

  // ---- measurement --------------------------------------------------------

  // The advance of one "0" in the paragraph's own face, metrics included. This
  // is what the CSS `ch` unit resolves to, so the number printed on the page is
  // the number a stylesheet would use.
  function chUnit(el, cs) {
    var probe = document.createElement('span');
    probe.textContent = '00';
    probe.style.cssText = [
      'position:absolute', 'top:0', 'left:-9999px', 'visibility:hidden',
      'white-space:pre', 'font-family:' + cs.fontFamily,
      'font-size:' + cs.fontSize, 'font-weight:' + cs.fontWeight,
      'font-style:' + cs.fontStyle,
      'letter-spacing:' + cs.letterSpacing,
      'word-spacing:' + cs.wordSpacing, 'font-feature-settings:' + cs.fontFeatureSettings
    ].join(';');
    document.body.appendChild(probe);
    var w = probe.getBoundingClientRect().width / 2;
    probe.remove();
    return w;
  }

  function measure() {
    var cs = getComputedStyle(para);
    var box = para.getBoundingClientRect();
    var unit = chUnit(para, cs);
    var leading = parseFloat(cs.lineHeight) || parseFloat(cs.fontSize) * 1.62;

    if (unit > 0) set('ch', Math.round(box.width / unit) + 'ch');
    set('lines', String(Math.max(1, Math.round(box.height / leading))));

    var headCs = getComputedStyle(head);
    var size = parseFloat(headCs.fontSize) || 0;
    var spacing = parseFloat(headCs.letterSpacing);
    if (size > 0 && !isNaN(spacing)) {
      set('track', (spacing / size).toFixed(3) + 'em');
    }

    // The ground this act paints. data-sc-drift is what the engine mixes the
    // page toward while the act owns the scroll, so it is the colour actually
    // behind this text, not the colour behind the hero.
    var ground = parseColor(act.getAttribute('data-sc-drift')) ||
                 parseColor(getComputedStyle(document.body).backgroundColor);
    var ink = parseColor(cs.color);

    if (ground && ink) {
      set('bghex', hex(ground));
      set('inkhex', hex(ink));
      set('ratio', ratio(ink, ground).toFixed(1) + ':1');

      var dotGround = root.querySelector('[data-audit-swatch="ground"]');
      var dotInk = root.querySelector('[data-audit-swatch="ink"]');
      if (dotGround) dotGround.style.background = hex(ground);
      if (dotInk) dotInk.style.background = hex(ink);
    }

    // ---- the lamps --------------------------------------------------------

    var housing = parseColor(getComputedStyle(lamp).backgroundColor);
    var keys = ['attn', 'working', 'ready'];

    if (lenses.length) {
      set('led', Math.round(lenses[0].getBoundingClientRect().width) + ' px');
    }

    if (!housing) return;

    Array.prototype.forEach.call(lenses, function (led, i) {
      var key = keys[i];
      if (!key) return;
      var colour = parseColor(getComputedStyle(led).backgroundColor);
      if (!colour) return;

      // The lens is measured at full brightness: the plate is a spec for the
      // state, not a reading of whichever lamp the scroll happens to be on.
      set('ratio-' + key, ratio(colour, housing).toFixed(1) + ':1');

      var dot = root.querySelector('[data-audit-swatch="' + key + '"]');
      if (dot) dot.style.background = hex(colour);
    });
  }

  // Fonts change the advance of a "0", so the run happens after the faces land.
  function run() {
    try { measure(); } catch (e) { /* a measurement must never break the page */ }
  }

  if (document.fonts && document.fonts.ready) {
    document.fonts.ready.then(run);
  }
  run();

  var timer = 0;
  window.addEventListener('resize', function () {
    clearTimeout(timer);
    timer = setTimeout(run, 160);
  }, { passive: true });
})();
