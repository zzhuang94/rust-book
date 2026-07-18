(function () {
  'use strict';

  var THEMES = [
    { id: 'light', icon: '☀️', label: '浅色' },
    { id: 'dark', icon: '🌙', label: '深色' },
    { id: 'sepia', icon: '📖', label: '护眼' },
  ];

  var FONT_SCALES = [0.75, 0.8, 0.85, 0.9, 0.95, 1, 1.05, 1.1, 1.15, 1.2, 1.3, 1.4];
  var DEFAULT_INDEX = 5;

  var KEY_THEME = 'doc-theme';
  var KEY_SCALE = 'doc-font-scale-idx';

  function loadPrefs() {
    var theme = localStorage.getItem(KEY_THEME) || 'light';
    var idx = parseInt(localStorage.getItem(KEY_SCALE), 10);
    if (isNaN(idx) || idx < 0 || idx >= FONT_SCALES.length) idx = DEFAULT_INDEX;
    return { theme: theme, scaleIndex: idx };
  }

  function savePrefs(prefs) {
    localStorage.setItem(KEY_THEME, prefs.theme);
    localStorage.setItem(KEY_SCALE, String(prefs.scaleIndex));
  }

  function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
  }

  function applyScale(index) {
    document.documentElement.style.setProperty('--font-scale', FONT_SCALES[index]);
  }

  function pct(index) {
    return Math.round(FONT_SCALES[index] * 100) + '%';
  }

  function buildToolbar() {
    var prefs = loadPrefs();
    applyTheme(prefs.theme);
    applyScale(prefs.scaleIndex);

    var el = document.createElement('div');
    el.className = 'doc-toolbar';
    el.innerHTML =
      '<div class="doc-toolbar-panel">' +
        '<div class="doc-toolbar-section">' +
          '<div class="doc-toolbar-label">主题</div>' +
          '<div class="doc-toolbar-row">' +
            THEMES.map(function (t) {
              return '<button class="doc-theme-btn' + (t.id === prefs.theme ? ' active' : '') +
                '" data-theme="' + t.id + '" title="' + t.label + '">' +
                '<span>' + t.icon + '</span>' +
                '<span class="theme-label">' + t.label + '</span>' +
                '</button>';
            }).join('') +
          '</div>' +
        '</div>' +
        '<div class="doc-toolbar-section">' +
          '<div class="doc-toolbar-label">字号</div>' +
          '<div class="doc-toolbar-row">' +
            '<button class="doc-font-btn" data-dir="-1" title="缩小字体">A-</button>' +
            '<span class="doc-font-display">' + pct(prefs.scaleIndex) + '</span>' +
            '<button class="doc-font-btn" data-dir="1" title="放大字体">A+</button>' +
          '</div>' +
        '</div>' +
      '</div>' +
      '<button class="doc-toolbar-toggle" title="显示设置">⚙</button>';

    document.body.appendChild(el);

    var toggle = el.querySelector('.doc-toolbar-toggle');
    toggle.addEventListener('click', function () {
      el.classList.toggle('expanded');
      // 展开设置面板时保持可见；收起后再走自动隐藏计时
      if (window.__docFabReveal) window.__docFabReveal();
    });

    document.addEventListener('click', function (e) {
      if (!el.contains(e.target)) {
        el.classList.remove('expanded');
        if (window.__docFabReveal) window.__docFabReveal();
      }
    });

    var themeBtns = el.querySelectorAll('.doc-theme-btn');
    themeBtns.forEach(function (btn) {
      btn.addEventListener('click', function () {
        prefs.theme = btn.getAttribute('data-theme');
        applyTheme(prefs.theme);
        savePrefs(prefs);
        themeBtns.forEach(function (b) {
          b.classList.toggle('active', b.getAttribute('data-theme') === prefs.theme);
        });
      });
    });

    var fontDisplay = el.querySelector('.doc-font-display');
    el.querySelectorAll('.doc-font-btn').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var dir = parseInt(btn.getAttribute('data-dir'), 10);
        var next = prefs.scaleIndex + dir;
        if (next < 0 || next >= FONT_SCALES.length) return;
        prefs.scaleIndex = next;
        applyScale(prefs.scaleIndex);
        savePrefs(prefs);
        fontDisplay.textContent = pct(prefs.scaleIndex);
      });
    });
  }

  /**
   * 手机端：两个悬浮钮默认隐藏；手指滑动/滚动时显示，静止约 1 秒后再藏起。
   * 桌面端不受影响。设置面板展开期间不自动隐藏。
   */
  function setupMobileFabAutoHide() {
    var mq = window.matchMedia('(max-width: 960px)');
    var HIDE_MS = 1000;
    var hideTimer = null;

    function isMobile() {
      return mq.matches;
    }

    function toolbarExpanded() {
      var tb = document.querySelector('.doc-toolbar');
      return !!(tb && tb.classList.contains('expanded'));
    }

    function clearHideTimer() {
      if (hideTimer !== null) {
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    }

    function hideFabs() {
      if (!isMobile()) return;
      if (toolbarExpanded()) return;
      document.documentElement.classList.add('fab-auto-hidden');
    }

    function revealFabs() {
      if (!isMobile()) {
        document.documentElement.classList.remove('fab-auto-hidden');
        clearHideTimer();
        return;
      }
      document.documentElement.classList.remove('fab-auto-hidden');
      clearHideTimer();
      if (toolbarExpanded()) return;
      hideTimer = setTimeout(hideFabs, HIDE_MS);
    }

    // 供设置按钮点击后复用
    window.__docFabReveal = revealFabs;

    function onScrollOrSwipe() {
      if (!isMobile()) return;
      revealFabs();
    }

    // capture：docsify 有时在 .content 内滚动
    window.addEventListener('scroll', onScrollOrSwipe, { passive: true, capture: true });
    document.addEventListener('touchmove', onScrollOrSwipe, { passive: true, capture: true });
    document.addEventListener('wheel', onScrollOrSwipe, { passive: true, capture: true });

    // 点到悬浮钮本身也刷新计时，避免刚摸到就被藏
    document.addEventListener(
      'pointerdown',
      function (e) {
        if (!isMobile()) return;
        var t = e.target;
        if (
          t &&
          t.closest &&
          (t.closest('.sidebar-toggle') || t.closest('.doc-toolbar'))
        ) {
          revealFabs();
        }
      },
      { passive: true }
    );

    function syncMode() {
      clearHideTimer();
      if (isMobile()) {
        // 进手机布局：先闪一下让用户知道钮在哪，再自动藏
        revealFabs();
      } else {
        document.documentElement.classList.remove('fab-auto-hidden');
      }
    }

    if (typeof mq.addEventListener === 'function') {
      mq.addEventListener('change', syncMode);
    } else if (typeof mq.addListener === 'function') {
      mq.addListener(syncMode);
    }

    syncMode();
  }

  function boot() {
    buildToolbar();
    setupMobileFabAutoHide();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
})();
