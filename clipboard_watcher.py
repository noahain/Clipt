"""
Clipboard Watcher - Monitors system clipboard for changes
"""

import pyperclip
import threading
import time
from datetime import datetime


class ClipboardWatcher:
    """Background service that monitors clipboard for new content"""

    POLL_INTERVAL = 2  # seconds

    def __init__(self, storage_manager):
        self.storage = storage_manager
        self.running = False
        self.paused = False
        self.last_content = None
        self._stop_event = threading.Event()
        self._pause_event = threading.Event()
        self._previous_date = datetime.now().strftime('%Y-%m-%d')

    def start(self):
        """Start the clipboard monitoring loop"""
        self.running = True
        self.last_content = self._get_clipboard_content()

        print("Clipboard watcher started")

        while not self._stop_event.is_set():
            try:
                if not self.paused:
                    self._check_clipboard()
                    self._check_date_change()

                # Sleep with interruptible wait
                self._stop_event.wait(timeout=self.POLL_INTERVAL)

            except Exception as e:
                print(f"Error in clipboard watcher: {e}")
                time.sleep(self.POLL_INTERVAL)

        print("Clipboard watcher stopped")

    def stop(self):
        """Stop the clipboard watcher"""
        self.running = False
        self._stop_event.set()

    def pause(self):
        """Pause clipboard monitoring"""
        self.paused = True
        self._pause_event.set()
        print("Clipboard watcher paused")

    def resume(self):
        """Resume clipboard monitoring"""
        self.paused = False
        self._pause_event.clear()
        # Reset last_content to avoid saving content that was copied while paused
        self.last_content = self._get_clipboard_content()
        print("Clipboard watcher resumed")

    def _get_clipboard_content(self):
        """Safely get clipboard content"""
        try:
            content = pyperclip.paste()
            return content if content else None
        except Exception as e:
            print(f"Error reading clipboard: {e}")
            return None

    def _check_clipboard(self):
        """Check for new clipboard content"""
        content = self._get_clipboard_content()

        if content is None:
            return

        # Skip if content hasn't changed
        if content == self.last_content:
            return

        # Skip empty content
        if not content.strip():
            return

        # Save to storage
        self._save_clip(content)
        self.last_content = content

    def _save_clip(self, content):
        """Save clipboard content to storage"""
        try:
            clip_id = self.storage.save_clip(content)
            timestamp = datetime.now().strftime('%H:%M:%S')
            content_preview = content[:50] + "..." if len(content) > 50 else content
            print(f"[{timestamp}] Clip saved (ID: {clip_id}): {content_preview}")
        except Exception as e:
            print(f"Error saving clip: {e}")

    def _check_date_change(self):
        """Check if date has changed and update current date"""
        current_date = datetime.now().strftime('%Y-%m-%d')
        if current_date != self._previous_date:
            print(f"Date changed from {self._previous_date} to {current_date}")
            self._previous_date = current_date
            self.last_content = None  # Reset to avoid cross-day duplication issues
