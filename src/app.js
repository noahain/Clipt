/**
 * Clipt UI — app.js (Tauri port)
 */
import { getVersion } from "@tauri-apps/api/app";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { checkForUpdates, initUpdater } from "./updater.js";

const GRADIENT_PRESETS = [
  'linear-gradient(135deg, #0a0a0a, #1a1a2e)',
  'linear-gradient(135deg, #0a0a0a, #2d1b00)',
  'linear-gradient(135deg, #0a0a0a, #1a0a2e)',
  'linear-gradient(135deg, #0a0a0a, #001f1f)',
  'linear-gradient(135deg, #0a0a0a, #2e1a0a)',
  'linear-gradient(135deg, #0a0a0a, #1a1a1a)',
  'linear-gradient(135deg, #0a0a0a 0%, #1a1a2e 50%, #0a0a0a 100%)',
  'radial-gradient(ellipse at center, #1a1a2e 0%, #0a0a0a 100%)',
];

class CliptApp {
  constructor() {
    this.currentDate = null;
    this.days = [];
    this.globalSearch = false;
    this.view = 'history';
    this.pins = [];
    this.pinKeys = new Set();
    this.pinTarget = null;
    this.theme = {
      themeMode: 'dark',
      ctaStyle: 'blur',
      bgType: 'none',
      bgColor: '#0a0a0a',
      bgGradient: '',
      bgTransparency: 75,
    };
    this.startup = {
      launchAtLogin: false,
      startMinimized: false,
    };

    this.el = {
      daysList:          document.getElementById('days-list'),
      clipsContainer:    document.getElementById('clips-container'),
      emptyState:        document.getElementById('empty-state'),
      historyView:       document.getElementById('history-view'),
      settingsView:      document.getElementById('settings-view'),
      currentDateTitle:  document.getElementById('current-date-title'),
      clipCount:         document.getElementById('clip-count'),
      searchInput:       document.getElementById('search-input'),
      searchScopeBtn:    document.getElementById('search-scope-btn'),
      searchWrap:        document.getElementById('search-wrap'),
      viewHistoryBtn:    document.getElementById('view-history-btn'),
      viewSettingsBtn:   document.getElementById('view-settings-btn'),
      refreshBtn:        document.getElementById('refresh-btn'),
      labelModal:        document.getElementById('label-modal'),
      labelInput:        document.getElementById('label-input'),
      pinModal:          document.getElementById('pin-modal'),
      pinNoteInput:      document.getElementById('pin-note-input'),
      pinTargetLabel:    document.getElementById('pin-target-label'),
      pinSaveBtn:        document.getElementById('pin-save-btn'),
      pinUnpinBtn:       document.getElementById('pin-unpin-btn'),
      toast:             document.getElementById('toast'),
      toastMessage:      document.getElementById('toast-message'),
      tbMenu:            document.getElementById('tb-menu'),
      tbMinimize:        document.getElementById('tb-minimize'),
      tbMaximize:        document.getElementById('tb-maximize'),
      tbClose:           document.getElementById('tb-close'),
      appMenu:           document.getElementById('app-menu'),
      menuPause:         document.getElementById('menu-pause'),
      menuRefresh:       document.getElementById('menu-refresh'),
      menuPins:          document.getElementById('menu-pins'),
      menuExit:          document.getElementById('menu-exit'),
      bgOverlay:         document.getElementById('bg-overlay'),
      themeModeSeg:      document.getElementById('theme-mode-seg'),
      ctaStyleSeg:       document.getElementById('cta-style-seg'),
      bgTypeSeg:         document.getElementById('bg-type-seg'),
      bgColorRow:        document.getElementById('bg-color-row'),
      bgColorInput:      document.getElementById('bg-color-input'),
      bgGradientRow:     document.getElementById('bg-gradient-row'),
      gradientGrid:      document.getElementById('gradient-grid'),
      bgTransparencyRow: document.getElementById('bg-transparency-row'),
      bgTransparency:    document.getElementById('bg-transparency'),
      bgTransparencyVal: document.getElementById('bg-transparency-val'),
      launchAtLoginSeg:  document.getElementById('launch-at-login-seg'),
      startMinimizedSeg: document.getElementById('start-minimized-seg'),
      appVersion:        document.getElementById('app-version'),
      checkUpdatesBtn:   document.getElementById('check-updates-btn'),
      updateStatus:      document.getElementById('update-status'),
    };

    this.editingDate = null;
    this.collapsedMonths = new Set();
    this.paused = false;
    this.init();
  }

  async init() {
    this.bindEvents();

    await this.loadSettings();
    await this.loadPins();
    await this.loadDays();
    this.setCurrentDate(this.getTodayDate());

    this.setupWatcherState();
    this.setupEvents();
    this.setupUpdater();
  }

  bindEvents() {
    this.el.refreshBtn.addEventListener('click', () => {
      this.loadDays();
      this.showToast('Refreshed');
    });

    this.el.labelInput.addEventListener('keydown', e => {
      if (e.key === 'Enter') this.saveLabel();
      if (e.key === 'Escape') this.closeLabelModal();
    });

    let searchTimer = null;
    this.el.searchInput.addEventListener('input', () => {
      clearTimeout(searchTimer);
      searchTimer = setTimeout(() => this.runSearch(this.el.searchInput.value.trim()), 250);
    });

    this.el.searchScopeBtn.addEventListener('click', () => {
      this.globalSearch = !this.globalSearch;
      this.el.searchScopeBtn.classList.toggle('active', this.globalSearch);
      if (this.isSearching()) this.runSearch(this.el.searchInput.value.trim());
    });

    this.el.viewHistoryBtn.addEventListener('click', () => this.showHistoryView());
    this.el.viewSettingsBtn.addEventListener('click', () => this.showSettingsView());

    this.el.pinNoteInput.addEventListener('keydown', e => {
      if (e.key === 'Enter') this.savePin();
      if (e.key === 'Escape') this.closePinModal();
    });

    this.el.themeModeSeg.addEventListener('click', e => {
      const btn = e.target.closest('[data-value]');
      if (btn) this.setThemeValue('themeMode', btn.dataset.value);
    });
    this.el.ctaStyleSeg.addEventListener('click', e => {
      const btn = e.target.closest('[data-value]');
      if (btn) this.setThemeValue('ctaStyle', btn.dataset.value);
    });
    this.el.bgTypeSeg.addEventListener('click', e => {
      const btn = e.target.closest('[data-value]');
      if (btn) this.setThemeValue('bgType', btn.dataset.value);
    });
    this.el.launchAtLoginSeg.addEventListener('click', e => {
      const btn = e.target.closest('[data-value]');
      if (btn) this.setStartupValue('launchAtLogin', btn.dataset.value === 'on');
    });
    this.el.startMinimizedSeg.addEventListener('click', e => {
      const btn = e.target.closest('[data-value]');
      if (btn) this.setStartupValue('startMinimized', btn.dataset.value === 'on');
    });
    this.el.bgColorInput.addEventListener('input', () => {
      this.theme.bgColor = this.el.bgColorInput.value;
      this.applyBackground();
    });
    this.el.bgColorInput.addEventListener('change', () => this.saveTheme());
    this.el.bgTransparency.addEventListener('input', () => {
      this.theme.bgTransparency = Number(this.el.bgTransparency.value);
      this.el.bgTransparencyVal.textContent = String(this.theme.bgTransparency);
      this.applyBackground();
    });
    this.el.bgTransparency.addEventListener('change', () => this.saveTheme());

    this.el.tbMinimize.addEventListener('click', () => getCurrentWindow().minimize());
    this.el.tbMaximize.addEventListener('click', () => getCurrentWindow().toggleMaximize());
    this.el.tbClose.addEventListener('click', () => getCurrentWindow().close());

    document.querySelector('#label-modal .id-btn-cancel').addEventListener('click', () => this.closeLabelModal());
    document.querySelector('#label-modal .id-btn-primary').addEventListener('click', () => this.saveLabel());
    document.querySelector('#pin-modal .id-btn-cancel').addEventListener('click', () => this.closePinModal());
    document.querySelector('#pin-modal .id-btn-danger').addEventListener('click', () => this.unpinCurrent());
    document.querySelector('#pin-modal .id-btn-primary').addEventListener('click', () => this.savePin());

    this.el.tbMenu.addEventListener('click', (e) => {
      e.stopPropagation();
      this.el.appMenu.classList.toggle('hidden');
    });
    document.addEventListener('click', (e) => {
      if (e.target.closest('#app-menu') || e.target.closest('#tb-menu')) return;
      this.el.appMenu.classList.add('hidden');
    });

    this.el.menuPause.addEventListener('click', async () => {
      this.el.appMenu.classList.add('hidden');
      try {
        await invoke('set_paused', { paused: !this.paused });
      } catch (err) {
        console.error('pause toggle failed:', err);
      }
    });
    this.el.menuRefresh.addEventListener('click', () => {
      this.el.appMenu.classList.add('hidden');
      this.loadDays();
      if (this.currentDate) this.loadDayData(this.currentDate);
      this.showToast('Refreshed');
    });
    this.el.menuExit.addEventListener('click', () => {
      invoke('exit_app');
    });
  }

  getTodayDate() {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  async loadDays() {
    try {
      const days = await invoke('get_days') || [];
      this.days = days.sort((a, b) => b.date.localeCompare(a.date));
      this.renderDaysList();
    } catch (err) {
      console.error('Failed to load days:', err);
      this.days = [];
      this.renderDaysList();
    }
  }

  renderDaysList() {
    const container = this.el.daysList;
    container.innerHTML = '';

    if (this.days.length === 0) {
      container.innerHTML = `<div class="sidebar-empty">No history yet</div>`;
      return;
    }

    const grouped = this.groupByMonth(this.days);

    Object.entries(grouped).forEach(([month, days]) => {
      const card = document.createElement('div');
      card.className = 'month-card';

      const header = document.createElement('button');
      header.className = 'month-card-header';
      header.innerHTML = `
        <span class="month-label">${this.escapeHtml(month)}</span>
        <span class="month-count">${days.length}</span>
        <svg class="month-chevron" width="10" height="10" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
        </svg>
      `;
      header.addEventListener('click', () => {
        if (this.collapsedMonths.has(month)) {
          this.collapsedMonths.delete(month);
        } else {
          this.collapsedMonths.add(month);
        }
        card.classList.toggle('collapsed', this.collapsedMonths.has(month));
      });

      const body = document.createElement('div');
      body.className = 'month-body';
      days.forEach(day => body.appendChild(this.createDayElement(day)));

      if (this.collapsedMonths.has(month)) card.classList.add('collapsed');

      card.appendChild(header);
      card.appendChild(body);
      container.appendChild(card);
    });
  }

  groupByMonth(days) {
    const grouped = {};
    days.forEach(day => {
      const d = new Date(day.date + 'T12:00:00');
      const key = d.toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
      if (!grouped[key]) grouped[key] = [];
      grouped[key].push(day);
    });
    return grouped;
  }

  createDayElement(day) {
    const isToday = day.date === this.getTodayDate();
    const isSelected = day.date === this.currentDate;
    const displayName = day.label || (isToday ? 'Today' : this.formatDate(day.date));

    const div = document.createElement('div');
    div.className = `day-item${isSelected ? ' active' : ''}`;

    const pinned = this.isPinned('day', day.date);
    div.innerHTML = `
      <div class="day-item-info">
        <div class="day-name">${this.escapeHtml(displayName)}</div>
        <div class="day-meta">
          ${day.label ? `<span class="day-date">${this.formatDate(day.date)}</span><span class="day-date">·</span>` : ''}
          <span class="day-count">${day.clip_count} clips</span>
        </div>
      </div>
      <button class="edit-label-btn" data-date="${day.date}" title="Edit label">
        <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
        </svg>
      </button>
      <button class="edit-label-btn pin-day-btn${pinned ? ' active' : ''}" data-date="${day.date}" title="${pinned ? 'Unpin day' : 'Pin day'}">
        <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 17v5"/>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>
        </svg>
      </button>
    `;

    div.addEventListener('click', e => {
      if (!e.target.closest('.edit-label-btn')) this.setCurrentDate(day.date);
    });

    div.querySelector('.pin-day-btn').addEventListener('click', e => {
      e.stopPropagation();
      this.openPinModal({ kind: 'day', day });
    });

    div.querySelector('.edit-label-btn:not(.pin-day-btn)').addEventListener('click', e => {
      e.stopPropagation();
      this.openLabelModal(day);
    });

    return div;
  }

  formatDate(dateStr) {
    return new Date(dateStr + 'T12:00:00').toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  async setCurrentDate(date) {
    this.currentDate = date;
    this.renderDaysList();
    const load = this.loadDayData(date);
    this.el.currentDateTitle.textContent = date === this.getTodayDate() ? 'Today' : this.formatDate(date);
    if (this.el.searchInput && this.el.searchInput.value) {
      this.el.searchInput.value = '';
    }
    await load;
  }

  async loadDayData(date) {
    try {
      const data = await invoke('get_day_data', { date }) || { clips: [] };
      this.renderClips(data.clips || []);
      const count = data.metadata?.clip_count ?? (data.clips || []).length;
      if (this.el.clipCount) this.el.clipCount.textContent = `${count} clips`;
    } catch (err) {
      console.error('Failed to load day data:', err);
      this.renderClips([]);
    }
  }

  async setupWatcherState() {
    try {
      const s = await invoke('get_watcher_state');
      this.updateWatcherState(!!s?.paused);
    } catch (err) {
      console.warn('watcher state unavailable:', err);
    }
  }

  async setupEvents() {
    try {
      await listen('clip-added', (e) => {
        const clip = e?.payload?.clip;
        if (this.isSearching()) {
          this.runSearch(this.el.searchInput.value.trim());
        } else if (clip && clip.date === this.currentDate) {
          this.loadDayData(this.currentDate);
        }
        this.loadDays();
      });
      await listen('clips-changed', (e) => {
        const date = e?.payload?.date || this.currentDate;
        if (this.isSearching()) {
          this.runSearch(this.el.searchInput.value.trim());
        } else if (date) {
          this.loadDayData(date);
        }
        this.loadDays();
      });
      await listen('watcher-state', (e) => {
        this.updateWatcherState(!!e?.payload?.paused);
      });
    } catch (err) {
      console.error('event subscribe failed:', err);
    }
  }

  isSearching() {
    return !!(this.el.searchInput && this.el.searchInput.value.trim());
  }

  updateWatcherState(paused) {
    this.paused = !!paused;
    if (this.el.menuPause) {
      this.el.menuPause.textContent = paused ? 'Resume' : 'Pause';
    }
  }

  renderClips(clips, showDate = false) {
    const container = this.el.clipsContainer;
    container.innerHTML = '';

    if (clips.length === 0) {
      this.el.emptyState.classList.remove('hidden');
      this.el.emptyState.style.display = 'flex';
      return;
    }

    this.el.emptyState.classList.add('hidden');
    this.el.emptyState.style.display = 'none';
    clips.forEach(clip => container.appendChild(this.createClipElement(clip, showDate)));
  }

  createClipElement(clip, showDate = false) {
    const div = document.createElement('div');
    div.className = 'clip-card';
    div.dataset.id = String(clip.id);

    const kind = clip.kind || 'text';
    const timeStr = this.clipTimeString(clip.timestamp);
    const dateBadge = showDate && clip.date ? `<span class="clip-date">${clip.date}</span>` : '';
    const actions = this.clipActionsHtml(this.isPinned('clip', clip.id));

    if (kind === 'image') {
      this.renderImageClip(div, clip, timeStr, dateBadge, actions);
    } else if (kind === 'files') {
      this.renderFilesClip(div, clip, timeStr, dateBadge, actions);
    } else {
      this.renderTextClip(div, clip, timeStr, dateBadge, actions);
    }
    this.attachClipActions(div, clip);
    return div;
  }

  clipTimeString(timestamp) {
    try {
      return new Date(timestamp).toLocaleTimeString('en-US', {
        hour: 'numeric', minute: '2-digit', hour12: true
      });
    } catch (e) {
      return '';
    }
  }

  clipActionsHtml(pinned) {
    return `
        <div class="clip-actions">
          <button class="copy-btn pin-btn${pinned ? ' active' : ''}" title="${pinned ? 'Unpin' : 'Pin'}">
            <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 17v5"/>
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>
            </svg>
          </button>
          <button class="copy-btn" title="Copy">
            <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
            </svg>
          </button>
          <button class="copy-btn delete-btn" title="Delete">
            <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
            </svg>
          </button>
        </div>`;
  }

  clipMetaHtml(timeStr, dateBadge, expandLabel = '') {
    return `
      <div class="clip-meta">
        <span class="clip-time">${timeStr}</span>
        ${dateBadge}
        ${expandLabel}
      </div>`;
  }

  renderImageClip(div, clip, timeStr, dateBadge, actions) {
    const src = clip.media_path ? convertFileSrc(clip.media_path) : '';
    div.innerHTML = `
      <div class="clip-card-header">
        <div class="clip-text">${src
          ? `<img src="${src}" alt="clipboard image" />`
          : `<span>[image missing]</span>`}</div>
        ${actions}
      </div>
      ${this.clipMetaHtml(timeStr, dateBadge)}
    `;
    const img = div.querySelector('img');
    if (img) {
      img.addEventListener('error', () => {
        const parent = img.parentElement;
        if (parent) parent.innerHTML = '<span>[image missing]</span>';
      });
    }
  }

  renderFilesClip(div, clip, timeStr, dateBadge, actions) {
    const paths = clip.file_paths || [];
    div.innerHTML = `
      <div class="clip-card-header">
        <div class="clip-text">${paths.map(p => this.escapeHtml(String(p))).join('<br>') || '[no files]'}</div>
        ${actions}
      </div>
      ${this.clipMetaHtml(timeStr, dateBadge)}
    `;
  }

  renderTextClip(div, clip, timeStr, dateBadge, actions) {
    const content = clip.text ?? clip.content ?? '';
    const isLong = content.length > 300;
    const displayContent = isLong ? content.substring(0, 300) + '...' : content;

    div.innerHTML = `
      <div class="clip-card-header">
        <div class="clip-text">${this.escapeHtml(displayContent)}</div>
        ${actions}
      </div>
      ${this.clipMetaHtml(timeStr, dateBadge, isLong ? '<span class="clip-expand">click to expand</span>' : '')}
    `;
    if (!isLong) return;

    let expanded = false;
    let downHadSelection = false;
    const selectionInCard = () => {
      const sel = window.getSelection();
      return !!(sel && !sel.isCollapsed && (div.contains(sel.anchorNode) || div.contains(sel.focusNode)));
    };
    div.addEventListener('mousedown', e => {
      downHadSelection = e.target.closest('.clip-actions') ? false : selectionInCard();
    });
    div.addEventListener('click', () => {
      const hadSelection = downHadSelection;
      downHadSelection = false;
      if (hadSelection || selectionInCard()) return;
      expanded = !expanded;
      div.querySelector('.clip-text').textContent = expanded ? content : displayContent;
      const ex = div.querySelector('.clip-expand');
      if (ex) ex.textContent = expanded ? 'click to collapse' : 'click to expand';
    });
  }

  attachClipActions(div, clip) {
    div.querySelector('.pin-btn').addEventListener('click', e => {
      e.stopPropagation();
      this.openPinModal({ kind: 'clip', clip });
    });
    div.querySelector('.copy-btn:not(.pin-btn)').addEventListener('click', e => {
      e.stopPropagation();
      this.copyClipById(clip.id);
    });
    div.querySelector('.delete-btn').addEventListener('click', e => {
      e.stopPropagation();
      this.deleteClipById(clip.id);
    });
  }

  async copyClipById(id) {
    try {
      const ok = await invoke('copy_clip', { id });
      this.showToast(ok ? 'Copied' : 'Copy failed');
    } catch (err) {
      this.showToast('Copy failed');
    }
  }

  async deleteClipById(id) {
    try {
      const ok = await invoke('delete_clip', { id });
      this.showToast(ok ? 'Deleted' : 'Delete failed');
    } catch (err) {
      this.showToast('Delete failed');
    }
  }

  async runSearch(query) {
    if (!query) {
      this.loadDayData(this.currentDate);
      this.el.currentDateTitle.textContent = this.currentDate === this.getTodayDate() ? 'Today' : this.formatDate(this.currentDate);
      return;
    }
    try {
      const date = this.globalSearch ? null : this.currentDate;
      const clips = await invoke('search_clips', { query, date }) || [];
      this.renderClips(clips, this.globalSearch);
      this.el.currentDateTitle.textContent = `${this.globalSearch ? 'Search (global)' : 'Search'}: "${query}"`;
      if (this.el.clipCount) this.el.clipCount.textContent = `${clips.length} results`;
    } catch (err) {
      console.error('Search failed:', err);
      this.renderClips([]);
    }
  }

  isPinned(kind, ref) {
    return this.pinKeys.has(`${kind}:${ref}`);
  }

  async loadPins() {
    try {
      this.pins = await invoke('get_pins') || [];
    } catch (err) {
      console.error('Failed to load pins:', err);
      this.pins = [];
    }
    this.pinKeys = new Set(this.pins.map(p => `${p.kind}:${p.ref}`));
    this.renderMenuPins();
  }

  pinPreviewFor(clip) {
    const kind = clip.kind || 'text';
    if (kind === 'image') return '[image]';
    if (kind === 'files') {
      const first = (clip.file_paths || [])[0] || '';
      const parts = String(first).split(/[\\/]/);
      return parts[parts.length - 1] || '[files]';
    }
    return (clip.text ?? clip.content ?? '').replace(/\s+/g, ' ').slice(0, 80);
  }

  openPinModal(target) {
    this.pinTarget = target;
    const isClip = target.kind === 'clip';
    const refId = isClip ? String(target.clip.id) : target.day.date;
    const preview = isClip
      ? this.pinPreviewFor(target.clip)
      : (target.day.label || this.formatDate(target.day.date));
    const existing = this.pins.find(p => p.kind === target.kind && p.ref === refId);
    this.el.pinTargetLabel.textContent = preview || (isClip ? 'Clip' : 'Day');
    this.el.pinNoteInput.value = existing ? existing.note : '';
    this.el.pinUnpinBtn.classList.toggle('hidden', !existing);
    this.el.pinSaveBtn.textContent = existing ? 'Save' : 'Pin';
    this.el.pinModal.classList.remove('hidden');
    setTimeout(() => this.el.pinNoteInput.focus(), 50);
  }

  closePinModal() {
    this.el.pinModal.classList.add('hidden');
    this.pinTarget = null;
  }

  async savePin() {
    const target = this.pinTarget;
    if (!target) return;
    const note = this.el.pinNoteInput.value.trim();
    let payload;
    if (target.kind === 'clip') {
      payload = {
        kind: 'clip',
        refId: String(target.clip.id),
        note,
        preview: this.pinPreviewFor(target.clip),
        date: target.clip.date,
      };
    } else {
      payload = {
        kind: 'day',
        refId: target.day.date,
        note,
        preview: target.day.label || this.formatDate(target.day.date),
        date: target.day.date,
      };
    }
    try {
      await invoke('set_pin', payload);
      await this.loadPins();
      this.refreshCurrentView();
      this.closePinModal();
      this.showToast('Pinned');
    } catch (err) {
      console.error('Pin failed:', err);
      this.showToast('Pin failed');
    }
  }

  async unpinCurrent() {
    const target = this.pinTarget;
    if (!target) return;
    const refId = target.kind === 'clip' ? String(target.clip.id) : target.day.date;
    try {
      await invoke('remove_pin', { kind: target.kind, refId });
      await this.loadPins();
      this.refreshCurrentView();
      this.closePinModal();
      this.showToast('Unpinned');
    } catch (err) {
      console.error('Unpin failed:', err);
      this.showToast('Unpin failed');
    }
  }

  refreshCurrentView() {
    this.renderDaysList();
    if (this.currentDate) this.loadDayData(this.currentDate);
  }

  renderMenuPins() {
    const el = this.el.menuPins;
    if (!el) return;
    if (this.pins.length === 0) {
      el.innerHTML = `<div class="menu-pins-label">Pins</div><div class="menu-pins-empty">No pins yet</div>`;
      return;
    }
    const pinSvg = `<svg width="11" height="11" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 17v5"/>
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>
    </svg>`;
    const items = this.pins.map((pin, i) => {
      const text = this.escapeHtml(pin.note || pin.preview || (pin.kind === 'day' ? pin.ref : 'Clip'));
      const meta = `${pin.kind === 'day' ? 'Day' : 'Clip'}${pin.date ? ' · ' + this.formatDate(pin.date) : ''}`;
      return `
        <button class="pin-item" data-pin-index="${i}">
          ${pinSvg}
          <span class="pin-item-text">
            <span class="pin-item-note">${text}</span>
            <span class="pin-item-meta">${this.escapeHtml(meta)}</span>
          </span>
        </button>`;
    }).join('');
    el.innerHTML = `<div class="menu-pins-label">Pins</div>${items}`;
    el.querySelectorAll('.pin-item').forEach(btn => {
      btn.addEventListener('click', () => {
        const pin = this.pins[Number(btn.dataset.pinIndex)];
        if (pin) this.openPin(pin);
      });
    });
  }

  async openPin(pin) {
    this.el.appMenu.classList.add('hidden');
    if (this.view === 'settings') this.showHistoryView();
    await this.setCurrentDate(pin.date);
    if (pin.kind !== 'clip') return;
    const card = document.querySelector(`.clip-card[data-id="${pin.ref}"]`);
    if (!card) {
      this.showToast('Clip no longer exists');
      return;
    }
    card.scrollIntoView({ block: 'center', behavior: 'smooth' });
    card.classList.add('clip-flash');
    setTimeout(() => card.classList.remove('clip-flash'), 1600);
  }

  showHistoryView() {
    this.view = 'history';
    this.el.historyView.classList.remove('hidden');
    this.el.settingsView.classList.add('hidden');
    this.el.viewHistoryBtn.classList.add('active');
    this.el.viewSettingsBtn.classList.remove('active');
    this.el.searchWrap.classList.remove('hidden');
  }

  showSettingsView() {
    this.view = 'settings';
    this.el.historyView.classList.add('hidden');
    this.el.settingsView.classList.remove('hidden');
    this.el.viewSettingsBtn.classList.add('active');
    this.el.viewHistoryBtn.classList.remove('active');
    this.el.searchWrap.classList.add('hidden');
    this.syncSettingsUI();
  }

  setupUpdater() {
    this.el.checkUpdatesBtn.addEventListener('click', () => this.checkUpdates());
    getVersion()
      .then(v => { this.el.appVersion.textContent = v; })
      .catch(err => console.warn('version unavailable:', err));
    initUpdater();
  }

  async checkUpdates() {
    this.el.checkUpdatesBtn.disabled = true;
    this.el.updateStatus.textContent = 'Checking...';
    try {
      const result = await checkForUpdates();
      if (result.installing) {
        this.el.updateStatus.textContent = 'Update in progress - see the prompt';
      } else if (result.pendingRestart) {
        this.el.updateStatus.textContent = 'Update installed - restart to finish';
      } else if (result.available) {
        this.el.updateStatus.textContent = `Version ${result.version} available`;
      } else {
        this.el.updateStatus.textContent = 'Up to date';
      }
    } catch (err) {
      this.el.updateStatus.textContent = 'Check failed';
      console.warn('update check failed:', err);
    }
    this.el.checkUpdatesBtn.disabled = false;
  }

  async loadSettings() {
    try {
      const s = await invoke('get_settings');
      if (s && s.theme) this.theme = { ...this.theme, ...s.theme };
      if (s) {
        this.startup.launchAtLogin = !!s.launchAtLogin;
        this.startup.startMinimized = !!s.startMinimized;
      }
    } catch (err) {
      console.warn('settings unavailable:', err);
    }
    if (this.theme.bgType === 'gradient' && !this.theme.bgGradient) {
      this.theme.bgGradient = GRADIENT_PRESETS[0];
    }
    this.applyTheme();
  }

  setThemeValue(key, value) {
    this.theme[key] = value;
    if (key === 'bgType' && value === 'gradient' && !this.theme.bgGradient) {
      this.theme.bgGradient = GRADIENT_PRESETS[0];
    }
    this.applyTheme();
    this.syncSettingsUI();
    this.saveTheme();
  }

  applyTheme() {
    const body = document.body;
    body.classList.add('theme-transitioning');
    body.classList.toggle('light-mode', this.theme.themeMode === 'light');
    body.classList.remove('cta-blur', 'cta-glass', 'cta-solid', 'cta-outline');
    body.classList.add(`cta-${this.theme.ctaStyle}`);
    this.applyBackground();
    clearTimeout(this._themeTimer);
    this._themeTimer = setTimeout(() => body.classList.remove('theme-transitioning'), 350);
  }

  applyBackground() {
    const o = this.el.bgOverlay;
    if (!o) return;
    const { bgType, bgColor, bgGradient, bgTransparency } = this.theme;
    if (bgType === 'none') {
      o.style.display = 'none';
      o.style.background = '';
      return;
    }
    o.style.display = 'block';
    o.style.background = bgType === 'color' ? bgColor : (bgGradient || GRADIENT_PRESETS[0]);
    o.style.opacity = String(bgTransparency / 100);
  }

  async saveTheme() {
    try {
      await invoke('update_settings', { patch: { theme: { ...this.theme } } });
    } catch (err) {
      console.warn('theme save failed:', err);
    }
  }

  setStartupValue(key, value) {
    this.startup[key] = value;
    this.syncSettingsUI();
    this.saveStartup();
  }

  async saveStartup() {
    try {
      await invoke('update_settings', { patch: { ...this.startup } });
    } catch (err) {
      console.warn('startup settings save failed:', err);
    }
  }

  syncSettingsUI() {
    const mark = (seg, value) => {
      seg.querySelectorAll('[data-value]').forEach(b => b.classList.toggle('active', b.dataset.value === value));
    };
    mark(this.el.themeModeSeg, this.theme.themeMode);
    mark(this.el.ctaStyleSeg, this.theme.ctaStyle);
    mark(this.el.bgTypeSeg, this.theme.bgType);
    mark(this.el.launchAtLoginSeg, this.startup.launchAtLogin ? 'on' : 'off');
    mark(this.el.startMinimizedSeg, this.startup.startMinimized ? 'on' : 'off');
    this.el.bgColorRow.classList.toggle('hidden', this.theme.bgType !== 'color');
    this.el.bgGradientRow.classList.toggle('hidden', this.theme.bgType !== 'gradient');
    this.el.bgTransparencyRow.classList.toggle('hidden', this.theme.bgType === 'none');
    this.el.bgColorInput.value = this.theme.bgColor;
    this.el.bgTransparency.value = String(this.theme.bgTransparency);
    this.el.bgTransparencyVal.textContent = String(this.theme.bgTransparency);
    this.renderGradientGrid();
  }

  renderGradientGrid() {
    const grid = this.el.gradientGrid;
    grid.innerHTML = GRADIENT_PRESETS.map((g, i) =>
      `<button class="gradient-swatch${this.theme.bgGradient === g ? ' active' : ''}" data-index="${i}" style="background-image:${g}"></button>`
    ).join('');
    grid.querySelectorAll('.gradient-swatch').forEach(btn => {
      btn.addEventListener('click', () => {
        this.theme.bgGradient = GRADIENT_PRESETS[Number(btn.dataset.index)];
        this.applyBackground();
        this.syncSettingsUI();
        this.saveTheme();
      });
    });
  }

  escapeHtml(text) {
    const d = document.createElement('div');
    d.textContent = text;
    return d.innerHTML;
  }

  openLabelModal(day) {
    this.editingDate = day.date;
    this.el.labelInput.value = day.label || '';
    this.el.labelModal.classList.remove('hidden');
    setTimeout(() => this.el.labelInput.focus(), 50);
  }

  closeLabelModal() {
    this.el.labelModal.classList.add('hidden');
    this.editingDate = null;
  }

  async saveLabel() {
    if (!this.editingDate) return;
    const label = this.el.labelInput.value.trim();
    try {
      await invoke('update_day_label', { date: this.editingDate, label });
      this.showToast('Label saved');
      this.closeLabelModal();
      this.loadDays();
    } catch (err) {
      this.showToast('Failed to save');
    }
  }

  showToast(message) {
    this.el.toastMessage.textContent = message;
    this.el.toast.classList.add('show');
    clearTimeout(this._toastTimer);
    this._toastTimer = setTimeout(() => this.el.toast.classList.remove('show'), 2500);
  }
}

function bootClipt() {
  new CliptApp();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', bootClipt);
} else {
  bootClipt();
}