/**
 * Clipt UI — app.js
 */

class CliptApp {
  constructor() {
    this.currentDate = null;
    this.days = [];
    this.isChatView = false;
    this.isStreaming = false;
    this.messageId = 0;
    this.sessionMessages = [];

    this.el = {
      daysList:          document.getElementById('days-list'),
      clipsContainer:    document.getElementById('clips-container'),
      emptyState:        document.getElementById('empty-state'),
      historyView:       document.getElementById('history-view'),
      chatView:          document.getElementById('chat-view'),
      chatMessages:      document.getElementById('chat-messages'),
      chatInput:         document.getElementById('chat-input'),
      sendBtn:           document.getElementById('send-btn'),
      currentDateTitle:  document.getElementById('current-date-title'),
      clipCount:         document.getElementById('clip-count'),
      viewHistoryBtn:    document.getElementById('view-history-btn'),
      chatBtn:           document.getElementById('chat-btn'),
      refreshBtn:        document.getElementById('refresh-btn'),
      labelModal:        document.getElementById('label-modal'),
      labelInput:        document.getElementById('label-input'),
      toast:             document.getElementById('toast'),
      toastMessage:      document.getElementById('toast-message'),
      statusText:        document.getElementById('status-text'),
    };

    this.editingDate = null;
    this.init();
  }

  async init() {
    this.bindEvents();

    // Wait for pywebview bridge to be ready
    let bridgeReady = false;
    for (let i = 0; i < 50; i++) {
      if (window.pywebview?.api?.get_days) {
        bridgeReady = true;
        break;
      }
      await new Promise(r => setTimeout(r, 100));
    }

    if (!bridgeReady) {
      console.warn('pywebview bridge not available, retrying...');
      // Try once more with longer wait
      await new Promise(r => setTimeout(r, 500));
    }

    await this.loadDays();
    this.setCurrentDate(this.getTodayDate());

    // Auto-refresh every 5s
    setInterval(() => this.loadDayData(this.currentDate), 5000);

    // Auto-resize textarea
    this.el.chatInput.addEventListener('input', () => {
      this.el.chatInput.style.height = 'auto';
      this.el.chatInput.style.height = Math.min(this.el.chatInput.scrollHeight, 120) + 'px';
    });
  }

  bindEvents() {
    this.el.refreshBtn.addEventListener('click', () => {
      this.loadDays();
      this.showToast('Refreshed');
    });

    this.el.viewHistoryBtn.addEventListener('click', () => this.showHistoryView());
    this.el.chatBtn.addEventListener('click', () => this.showChatView());

    this.el.labelInput.addEventListener('keydown', e => {
      if (e.key === 'Enter') this.saveLabel();
      if (e.key === 'Escape') this.closeLabelModal();
    });

    this.el.chatInput.addEventListener('keydown', e => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        this.sendMessage();
      }
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
      const days = await window.pywebview?.api?.get_days() || [];
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
      container.innerHTML = `<div style="padding:20px;text-align:center;color:#444;font-size:12px;">No history yet</div>`;
      return;
    }

    const grouped = this.groupByMonth(this.days);

    Object.entries(grouped).forEach(([month, days]) => {
      const group = document.createElement('div');
      group.className = 'month-group';

      const label = document.createElement('div');
      label.className = 'month-label';
      label.textContent = month;
      group.appendChild(label);

      days.forEach(day => group.appendChild(this.createDayElement(day)));
      container.appendChild(group);
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
    `;

    div.addEventListener('click', e => {
      if (!e.target.closest('.edit-label-btn')) this.setCurrentDate(day.date);
    });

    div.querySelector('.edit-label-btn').addEventListener('click', e => {
      e.stopPropagation();
      this.openLabelModal(day);
    });

    return div;
  }

  formatDate(dateStr) {
    return new Date(dateStr + 'T12:00:00').toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  setCurrentDate(date) {
    this.currentDate = date;
    this.renderDaysList();
    this.loadDayData(date);
    this.el.currentDateTitle.textContent = date === this.getTodayDate() ? 'Today' : this.formatDate(date);
  }

  async loadDayData(date) {
    try {
      const data = await window.pywebview?.api?.get_day_data(date) || { clips: [] };
      this.renderClips(data.clips);
      this.el.clipCount.textContent = `${data.clips.length} clips`;
    } catch (err) {
      console.error('Failed to load day data:', err);
      this.renderClips([]);
    }
  }

  renderClips(clips) {
    const container = this.el.clipsContainer;
    container.innerHTML = '';

    if (clips.length === 0) {
      this.el.emptyState.classList.remove('hidden');
      this.el.emptyState.style.display = 'flex';
      return;
    }

    this.el.emptyState.classList.add('hidden');
    this.el.emptyState.style.display = 'none';
    clips.forEach(clip => container.appendChild(this.createClipElement(clip)));
  }

  createClipElement(clip) {
    const div = document.createElement('div');
    div.className = 'clip-card';

    let timeStr = '';
    try {
      timeStr = new Date(clip.timestamp).toLocaleTimeString('en-US', {
        hour: 'numeric', minute: '2-digit', hour12: true
      });
    } catch (e) {}

    const content = clip.content;
    const isLong = content.length > 300;
    const displayContent = isLong ? content.substring(0, 300) + '...' : content;

    div.innerHTML = `
      <div class="clip-card-header">
        <div class="clip-text">${this.escapeHtml(displayContent)}</div>
        <button class="copy-btn" title="Copy">
          <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
          </svg>
        </button>
      </div>
      <div class="clip-meta">
        <span class="clip-time">${timeStr}</span>
        ${isLong ? '<span class="clip-expand">click to expand</span>' : ''}
      </div>
    `;

    div.querySelector('.copy-btn').addEventListener('click', e => {
      e.stopPropagation();
      this.copyToClipboard(content);
    });

    let expanded = false;
    if (isLong) {
      div.addEventListener('click', () => {
        expanded = !expanded;
        div.querySelector('.clip-text').textContent = expanded ? content : displayContent;
        div.querySelector('.clip-expand').textContent = expanded ? 'click to collapse' : 'click to expand';
      });
    }

    return div;
  }

  escapeHtml(text) {
    const d = document.createElement('div');
    d.textContent = text;
    return d.innerHTML;
  }

  async copyToClipboard(text) {
    try {
      // Use Python API to write to real Windows clipboard — avoids pywebview sandbox
      const result = await window.pywebview?.api?.write_clipboard(text);
      if (result) {
        this.showToast('Copied');
        return;
      }
    } catch (e) {
      console.warn('Python clipboard write failed, trying browser API:', e);
    }
    // Fallback
    try {
      await navigator.clipboard.writeText(text);
      this.showToast('Copied');
    } catch (err) {
      this.showToast('Copy failed');
    }
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
      await window.pywebview?.api?.update_day_label(this.editingDate, label);
      this.showToast('Label saved');
      this.closeLabelModal();
      this.loadDays();
    } catch (err) {
      this.showToast('Failed to save');
    }
  }

  showHistoryView() {
    this.isChatView = false;
    this.el.historyView.classList.remove('hidden');
    this.el.historyView.style.display = 'block';
    this.el.chatView.classList.add('hidden');
    this.el.chatView.style.display = 'none';
    this.el.viewHistoryBtn.classList.add('active');
    this.el.chatBtn.classList.remove('active');
  }

  showChatView() {
    this.isChatView = true;
    this.el.historyView.classList.add('hidden');
    this.el.historyView.style.display = 'none';
    this.el.chatView.classList.remove('hidden');
    this.el.chatView.style.display = 'flex';
    this.el.chatBtn.classList.add('active');
    this.el.viewHistoryBtn.classList.remove('active');
    this.el.chatInput.focus();
  }

  async sendMessage() {
    if (this.isStreaming) return;
    const message = this.el.chatInput.value.trim();
    if (!message) return;

    this.addMessage('user', message);
    this.sessionMessages.push({ role: 'user', content: message, timestamp: new Date().toISOString() });
    this.el.chatInput.value = '';
    this.el.chatInput.style.height = 'auto';
    await this.streamResponse(message);
  }

  addMessage(role, content) {
    const isUser = role === 'user';
    const row = document.createElement('div');
    row.className = `message-row ${isUser ? 'user' : 'ai'}`;

    const avatarHtml = isUser
      ? `<div class="msg-avatar user">U</div>`
      : `<div class="msg-avatar ai"><img src="assets/icon.png" alt="C"></div>`;

    let bubbleContent;
    if (isUser) {
      bubbleContent = `<p class="selectable" style="white-space:pre-wrap;word-break:break-word;">${this.escapeHtml(content)}</p>`;
    } else {
      const processedContent = this.processMarkdownWithCopyButtons(content);
      bubbleContent = `<div class="markdown-content">${processedContent}</div>`;
    }

    row.innerHTML = `${avatarHtml}<div class="chat-bubble">${bubbleContent}</div>`;
    this.el.chatMessages.appendChild(row);

    // Attach event listeners to copy buttons
    if (!isUser) {
      row.querySelectorAll('.ai-copy-btn').forEach(btn => {
        btn.addEventListener('click', e => this.copyAiCode(e.currentTarget));
      });
    }

    this.scrollToBottom();
    return row;
  }

  processMarkdownWithCopyButtons(content) {
    // Parse markdown
    const html = marked.parse(content);

    // Wrap code blocks with container and add copy button
    return html.replace(
      /<pre><code([^>]*)>([\s\S]*?)<\/code><\/pre>/g,
      (match, codeAttrs, codeContent) => {
        // Get language from class
        const langMatch = codeAttrs.match(/class="[^"]*language-(\w+)"|<code class="language-(\w+)"/);
        const lang = langMatch ? (langMatch[1] || langMatch[2]) : 'code';

        // Decode HTML entities for the data attribute
        const decodedCode = codeContent
          .replace(/&lt;/g, '<')
          .replace(/&gt;/g, '>')
          .replace(/&amp;/g, '&');

        return `<div class="ai-code-block">
          <div class="ai-code-header">
            <span class="ai-code-lang">${lang}</span>
            <button class="ai-copy-btn" data-code="${this.escapeHtml(decodedCode)}" title="Copy code">
              <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
              </svg>
              Copy
            </button>
          </div>
          <pre><code${codeAttrs}>${codeContent}</code></pre>
        </div>`;
      }
    );
  }

  async copyAiCode(button) {
    const code = button.getAttribute('data-code');
    if (!code) return;

    // Visual feedback
    const originalText = button.innerHTML;
    button.classList.add('copied');
    button.innerHTML = `
      <svg width="12" height="12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
      </svg>
      Copied!
    `;

    await this.copyToClipboard(code);

    // Reset after delay
    setTimeout(() => {
      button.classList.remove('copied');
      button.innerHTML = originalText;
    }, 2000);
  }

  async streamResponse(query) {
    if (!this.currentDate) {
      this.addMessage('assistant', 'Please select a day from the sidebar first.');
      return;
    }

    this.isStreaming = true;
    this.el.sendBtn.disabled = true;

    // Thinking placeholder
    const row = document.createElement('div');
    row.className = 'message-row ai';
    row.innerHTML = `
      <div class="msg-avatar ai"><img src="assets/icon.png" alt="C"></div>
      <div class="chat-bubble">
        <p class="streaming-content" style="color:#555;">Thinking...</p>
      </div>
    `;
    this.el.chatMessages.appendChild(row);
    this.scrollToBottom();

    const contentEl = row.querySelector('.streaming-content');
    this.messageId++;

    try {
      const sessionHistoryJson = JSON.stringify(this.sessionMessages.slice(-10));
      const response = await window.pywebview?.api?.chat_with_history(
        this.currentDate,
        query,
        this.messageId,
        sessionHistoryJson
      );

      const responseText = typeof response === 'string' ? response : JSON.stringify(response);
      this.sessionMessages.push({ role: 'assistant', content: responseText, timestamp: new Date().toISOString() });

      const bubble = row.querySelector('.chat-bubble');
      const processedContent = this.processMarkdownWithCopyButtons(responseText);
      bubble.innerHTML = `<div class="markdown-content">${processedContent}</div>`;

      // Attach event listeners to copy buttons
      bubble.querySelectorAll('.ai-copy-btn').forEach(btn => {
        btn.addEventListener('click', e => this.copyAiCode(e.currentTarget));
      });
    } catch (err) {
      console.error('Chat error:', err);
      contentEl.textContent = 'Something went wrong. Check your NVIDIA NIM API key.';
      contentEl.style.color = '#f87171';
    } finally {
      this.isStreaming = false;
      this.el.sendBtn.disabled = false;
      this.scrollToBottom();
    }
  }

  scrollToBottom() {
    this.el.chatMessages.scrollTop = this.el.chatMessages.scrollHeight;
  }

  startNewChat() {
    const count = this.sessionMessages.length;
    this.sessionMessages = [];
    // Clear all messages except welcome
    const msgs = this.el.chatMessages.querySelectorAll(':scope > div');
    msgs.forEach((m, i) => { if (i > 0) m.remove(); });
    this.showToast(`Cleared ${count} messages`);
  }

  showToast(message) {
    this.el.toastMessage.textContent = message;
    this.el.toast.classList.add('show');
    clearTimeout(this._toastTimer);
    this._toastTimer = setTimeout(() => this.el.toast.classList.remove('show'), 2500);
  }
}

document.addEventListener('DOMContentLoaded', () => {
  window.app = new CliptApp();
});
