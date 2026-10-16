import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

const CHECK_TIMEOUT_MS = 10000;
const AUTO_RETRY_DELAY_MS = 5000;

let activeUpdate = null;
let installing = false;
let pendingRestart = false;
let downloaded = 0;
let contentLength = 0;
let toast = null;
let toastEls = null;

function classifyError(err) {
  const message = String((err && err.message) || err);
  if (/signature|verify/i.test(message)) return 'Signature verification failed - update rejected';
  if (/rate.?limit|403/i.test(message)) return 'GitHub rate limit - try again later';
  if (/not.?found|404/i.test(message)) return 'Update manifest not found';
  if (/network|fetch|timeout|tls|dns|connect/i.test(message)) return 'Network error - check your connection';
  return 'Update failed';
}

function ensureToast() {
  if (toast) return;
  toast = document.createElement('div');
  toast.id = 'updater-toast';
  toast.innerHTML = `
    <div class="update-toast-header">
      <span class="update-toast-title"></span>
      <button class="update-toast-close" title="Dismiss">&times;</button>
    </div>
    <p class="update-toast-text"></p>
    <div class="update-progress hidden"><div class="update-progress-fill"></div></div>
    <div class="update-toast-actions">
      <button class="id-btn-primary update-btn-primary hidden"></button>
      <button class="id-btn-cancel update-btn-secondary hidden"></button>
    </div>
  `;
  document.body.appendChild(toast);
  toastEls = {
    title: toast.querySelector('.update-toast-title'),
    close: toast.querySelector('.update-toast-close'),
    text: toast.querySelector('.update-toast-text'),
    progress: toast.querySelector('.update-progress'),
    fill: toast.querySelector('.update-progress-fill'),
    primary: toast.querySelector('.update-btn-primary'),
    secondary: toast.querySelector('.update-btn-secondary'),
  };
  toastEls.close.addEventListener('click', () => hideToast());
}

function render({ title, text, primary, secondary, progress }) {
  ensureToast();
  toastEls.title.textContent = title;
  toastEls.text.textContent = text;
  if (progress === null) {
    toastEls.progress.classList.add('hidden');
    toast.classList.remove('indeterminate');
    toastEls.fill.style.width = '0%';
  } else {
    toastEls.progress.classList.remove('hidden');
    toast.classList.toggle('indeterminate', progress === 'indeterminate');
    if (typeof progress === 'number') toastEls.fill.style.width = `${progress}%`;
  }
  for (const [button, spec] of [[toastEls.primary, primary], [toastEls.secondary, secondary]]) {
    if (!spec) {
      button.classList.add('hidden');
      button.onclick = null;
      continue;
    }
    button.textContent = spec.label;
    button.onclick = spec.onClick;
    button.classList.remove('hidden');
  }
  toast.classList.add('show');
}

function hideToast() {
  if (toast) toast.classList.remove('show');
}

function showAvailable() {
  render({
    title: 'Update available',
    text: `Clipt ${activeUpdate.version} is ready to install.`,
    primary: { label: 'Install', onClick: startInstall },
    secondary: { label: 'Later', onClick: hideToast },
    progress: null,
  });
}

async function startInstall() {
  if (installing || !activeUpdate) return;
  installing = true;
  downloaded = 0;
  contentLength = 0;
  render({
    title: 'Updating',
    text: 'Starting download...',
    primary: null,
    secondary: null,
    progress: 'indeterminate',
  });
  try {
    await activeUpdate.downloadAndInstall(event => {
      if (event.event === 'Started') {
        contentLength = (event.data && event.data.contentLength) || 0;
        render({
          title: 'Updating',
          text: 'Downloading...',
          primary: null,
          secondary: null,
          progress: contentLength ? 0 : 'indeterminate',
        });
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength;
        const pct = contentLength ? Math.min(100, Math.round((downloaded / contentLength) * 100)) : null;
        render({
          title: 'Updating',
          text: pct === null ? 'Downloading...' : `Downloading... ${pct}%`,
          primary: null,
          secondary: null,
          progress: pct === null ? 'indeterminate' : pct,
        });
      } else if (event.event === 'Finished') {
        render({
          title: 'Updating',
          text: 'Installing - the app will restart',
          primary: null,
          secondary: null,
          progress: 100,
        });
      }
    });
    installing = false;
    pendingRestart = true;
    activeUpdate.close().catch(() => {});
    activeUpdate = null;
    render({
      title: 'Update ready',
      text: 'Restart to run the new version.',
      primary: { label: 'Restart', onClick: () => relaunch() },
      secondary: { label: 'Later', onClick: hideToast },
      progress: null,
    });
  } catch (err) {
    installing = false;
    render({
      title: 'Update failed',
      text: classifyError(err),
      primary: { label: 'Retry', onClick: startInstall },
      secondary: { label: 'Dismiss', onClick: hideToast },
      progress: null,
    });
    console.warn('update install failed:', err);
  }
}

async function autoCheck(retried) {
  try {
    await checkForUpdates();
  } catch (err) {
    console.warn('update check failed:', err);
    if (!retried) setTimeout(() => autoCheck(true), AUTO_RETRY_DELAY_MS);
  }
}

export function initUpdater() {
  if (!import.meta.env.PROD) return;
  autoCheck(false);
}

export async function checkForUpdates() {
  if (installing) {
    return { available: true, installing: true, pendingRestart: false, version: activeUpdate.version };
  }
  if (pendingRestart) {
    return { available: true, installing: false, pendingRestart: true, version: null };
  }
  const next = await check({ timeout: CHECK_TIMEOUT_MS });
  if (activeUpdate) activeUpdate.close().catch(() => {});
  activeUpdate = next;
  if (!next) {
    hideToast();
    return { available: false, installing: false, pendingRestart: false, version: null };
  }
  downloaded = 0;
  contentLength = 0;
  showAvailable();
  return { available: true, installing: false, pendingRestart: false, version: next.version };
}
