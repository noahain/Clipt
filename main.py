#!/usr/bin/env python3
"""
Clipt - Clipboard History Manager with AI Chat
Main entry point with safe hiding and detailed logging
"""

import sys
import socket
import threading
import time
import traceback
from datetime import datetime
from pathlib import Path
import os
import argparse

# Must be before any other imports to catch second instance early
SHOW_COMMAND = b"SHOW"
WAKE_PORT = 47200

def log(msg):
    """Print with timestamp for debugging"""
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    print(f"[{ts}] {msg}")

def try_wake_existing_instance():
    """Try to wake existing instance, return True if successful"""
    try:
        log("Checking for existing instance...")
        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client.settimeout(0.5)
        client.connect(('127.0.0.1', WAKE_PORT))
        client.send(SHOW_COMMAND)
        client.close()
        log("Sent SHOW command to existing instance")
        return True
    except:
        return False

# Check early if another instance is running
if try_wake_existing_instance():
    print("Clipt is already running. Waking up existing instance...")
    sys.exit(0)

log("StartingCliPt as first instance")

import webview

project_root = Path(__file__).parent
sys.path.insert(0, str(project_root))

from storage_manager import StorageManager
from clipboard_watcher import ClipboardWatcher
from tray_icon import TrayIcon
from ai_handler import AIHandler


class CliptApp:

    def __init__(self, startup_mode=False):
        log(f"Initializing CliptApp (startup_mode={startup_mode})")
        self.running = False
        self.startup_mode = startup_mode
        self.storage = StorageManager()
        self.watcher = ClipboardWatcher(self.storage)
        self.tray = TrayIcon(self)
        self.ai = AIHandler(self.storage)
        self.window = None
        self.wakeup_socket = None
        self._window_visible = not startup_mode
        self.tray_menu_window = None

    def start(self):
        """Start all components"""
        log("Starting components...")
        self.running = True

        # Start wake-up listener first
        log("Starting wake-up listener...")
        self._start_wakeup_listener()

        # Start clipboard watcher
        log("Starting clipboard watcher...")
        threading.Thread(
            target=self.watcher.start,
            name="ClipboardWatcher",
            daemon=True
        ).start()

        # Start tray icon
        log("Starting tray icon...")
        threading.Thread(
            target=self.tray.run,
            name="TrayIcon",
            daemon=True
        ).start()

        # Wait a moment
        time.sleep(0.5)

        # Create and show window
        log("Creating window...")
        self._create_and_show_window()

    def _start_wakeup_listener(self):
        """Start socket listener for wake-up commands"""
        def listen():
            self.wakeup_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.wakeup_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

            try:
                self.wakeup_socket.bind(('127.0.0.1', WAKE_PORT))
                self.wakeup_socket.listen(1)
                self.wakeup_socket.settimeout(1.0)
                log(f"Wake-up listener bound to port {WAKE_PORT}")
            except socket.error as e:
                log(f"ERROR: Failed to bind wake-up port: {e}")
                return

            while self.running:
                try:
                    conn, addr = self.wakeup_socket.accept()
                    try:
                        data = conn.recv(10)
                        if data == SHOW_COMMAND:
                            log("[WAKEUP] Received SHOW command")
                            self._safe_show()
                    except:
                        pass
                    finally:
                        conn.close()
                except socket.timeout:
                    continue
                except Exception as e:
                    if self.running:
                        log(f"[WAKEUP] Error: {e}")

        threading.Thread(target=listen, daemon=True).start()

    def _safe_show(self):
        """Thread-safe window show"""
        log("[DEBUG] _safe_show called")
        if not self.window:
            log("[ERROR] Window is None")
            return
        try:
            log("[DEBUG] Calling window.show()...")
            self.window.show()
            log("[DEBUG] Calling window.restore()...")
            self.window.restore()
            self._window_visible = True
            log("[DEBUG] Window shown successfully")
        except Exception as e:
            log(f"[ERROR] Show failed: {e}")

    def _create_and_show_window(self):
        """Create window and start webview"""
        ui_path = Path(__file__).parent / "ui" / "index.html"
        app_data = Path(os.environ.get('APPDATA', Path.home() / "AppData" / "Roaming")) / "Clipt" / "webview_cache"
        app_data.mkdir(parents=True, exist_ok=True)

        log(f"[DEBUG] Creating window with cache at: {app_data}")

        # Define the closing handler with deferred hide
        def on_closing():
            """Defer hide to avoid deadlock"""
            log("[DEBUG] ======================================")
            log("[DEBUG] Close event triggered on main thread")
            log("[DEBUG] Scheduling deferred hide via Timer...")

            # DEFERRED HIDE: Use timer to avoid deadlock
            def do_hide():
                log("[DEBUG] Executing deferred hide...")
                try:
                    self.window.hide()
                    self._window_visible = False
                    log("[DEBUG] Window hidden successfully")
                except Exception as e:
                    log(f"[ERROR] Hide failed: {e}")
                log("[DEBUG] ======================================")

            threading.Timer(0.1, do_hide).start()
            log("[DEBUG] Timer started, returning False immediately")
            return False  # Prevent destruction

        try:
            log("[DEBUG] Calling webview.create_window...")
            self.window = webview.create_window(
                title='Clipt',
                url=str(ui_path),
                width=1200,
                height=800,
                min_size=(900, 600),
                confirm_close=False,
                hidden=self.startup_mode
            )
            log("[DEBUG] Window created successfully")

            # Register closing handler BEFORE exposing API
            log("[DEBUG] Registering closing event handler...")
            self.window.events.closing += on_closing
            log("[DEBUG] Closing handler registered")

            # Expose API
            log("[DEBUG] Exposing Python API...")
            self.window.expose(self.get_days)
            self.window.expose(self.get_day_data)
            self.window.expose(self.update_day_label)
            self.window.expose(self.chat_with_history)
            self.window.expose(self.get_today_context)
            self.window.expose(self.write_clipboard)
            log("[DEBUG] API exposed")

            self._window_visible = not self.startup_mode
            log("[DEBUG] Starting webview loop...")

            # Start webview - this blocks
            try:
                webview.start(
                    debug=False,
                    http_server=True,
                    http_port=8765,
                    gui='default'  # Use system default (WebView2 on Windows)
                )
                log("[DEBUG] Webview loop exited normally")
            except Exception as e:
                log(f"[FATAL] Webview.start() crashed: {e}")
                log(traceback.format_exc())

        except Exception as e:
            log(f"[FATAL] Failed to create window: {e}")
            log(traceback.format_exc())

    def open_ui(self):
        """Show UI from tray"""
        log("[DEBUG] open_ui called from tray")
        self._safe_show()

    def show_tray_menu(self, x=None, y=None):
        """Show custom styled tray menu"""
        log(f"[DEBUG] show_tray_menu called at ({x}, {y})")

        # Close existing menu if open
        if self.tray_menu_window:
            try:
                self.tray_menu_window.destroy()
            except:
                pass
            self.tray_menu_window = None

        # Get mouse position if not provided
        if x is None or y is None:
            try:
                import pyautogui
                x, y = pyautogui.position()
            except:
                x, y = 100, 100

        # Ensure menu stays on screen
        screen_width, screen_height = 1920, 1080  # Default, ideally detect actual screen size
        menu_width, menu_height = 160, 150

        # Adjust position to keep menu on screen
        if x + menu_width > screen_width:
            x = screen_width - menu_width - 10
        if y + menu_height > screen_height:
            y = y - menu_height

        menu_path = Path(__file__).parent / "ui" / "tray_menu.html"

        try:
            # Create frameless popup window
            self.tray_menu_window = webview.create_window(
                title='Menu',
                url=str(menu_path),
                width=menu_width,
                height=menu_height,
                x=x,
                y=y,
                frameless=True,
                on_top=True,
                confirm_close=False
            )

            # Expose menu API
            class MenuAPI:
                def __init__(self, app):
                    self.app = app

                def open(self):
                    log("[MENU] Open clicked")
                    self.app._close_tray_menu()
                    self.app.open_ui()
                    return True

                def settings(self):
                    log("[MENU] Settings clicked")
                    # Placeholder for settings
                    self.app._close_tray_menu()
                    return True

                def exit_app(self):
                    log("[MENU] Exit clicked")
                    self.app.shutdown()
                    return True

                def close(self):
                    self.app._close_tray_menu()
                    return True

            menu_api = MenuAPI(self)
            self.tray_menu_window.expose(menu_api.open)
            self.tray_menu_window.expose(menu_api.settings)
            self.tray_menu_window.expose(menu_api.exit_app)
            self.tray_menu_window.expose(menu_api.close)

            log("[DEBUG] Tray menu window created")

            # Auto-close after 5 seconds if not interacted
            threading.Timer(5.0, self._auto_close_tray_menu).start()

        except Exception as e:
            log(f"[ERROR] Failed to create tray menu: {e}")
            self.tray_menu_window = None

    def _close_tray_menu(self):
        """Close the tray menu window"""
        if self.tray_menu_window:
            try:
                self.tray_menu_window.destroy()
                log("[DEBUG] Tray menu closed")
            except:
                pass
            self.tray_menu_window = None

    def _auto_close_tray_menu(self):
        """Auto-close menu after timeout"""
        self._close_tray_menu()

    def pause_watcher(self):
        log("Pausing watcher...")
        self.watcher.pause()
        self.tray.update_menu(paused=True)

    def resume_watcher(self):
        log("Resuming watcher...")
        self.watcher.resume()
        self.tray.update_menu(paused=False)

    def shutdown(self):
        """Hard shutdown"""
        log("Shutting down Clipt...")
        self.running = False
        if self.wakeup_socket:
            try:
                self.wakeup_socket.close()
            except:
                pass
        self.watcher.stop()
        self.tray.stop()
        log("Goodbye!")
        os._exit(0)

    # ── API methods ──────────────────────────────────────────

    def get_days(self):
        return self.storage.get_all_days()

    def get_day_data(self, date_str):
        return self.storage.get_day_data(date_str)

    def update_day_label(self, date_str, label):
        self.storage.update_day_label(date_str, label)
        return True

    def chat_with_history(self, date_str, query, message_id=None, session_history=None):
        return self.ai.chat_with_history(date_str, query, message_id, session_history)

    def get_today_context(self):
        from datetime import datetime
        today = datetime.now().strftime('%Y-%m-%d')
        return self.storage.get_day_data(today)

    def write_clipboard(self, text):
        """Write text to Windows clipboard without flashing window"""
        try:
            import subprocess
            import sys

            if sys.platform == 'win32':
                # Hide PowerShell window to prevent flash
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                startupinfo.wShowWindow = subprocess.SW_HIDE

                subprocess.run(
                    ['powershell', '-command', f'Set-Clipboard -Value {repr(text)}'],
                    capture_output=True,
                    timeout=3,
                    startupinfo=startupinfo
                )
            else:
                subprocess.run(
                    ['powershell', '-command', f'Set-Clipboard -Value {repr(text)}'],
                    capture_output=True,
                    timeout=3
                )
            return True
        except Exception as e:
            try:
                import pyperclip
                pyperclip.copy(text)
                return True
            except:
                return False


def main():
    parser = argparse.ArgumentParser(description='Clipt - Clipboard History Manager')
    parser.add_argument('--startup', action='store_true', help='Launch in system tray only')
    args = parser.parse_args()

    app = CliptApp(startup_mode=args.startup)
    try:
        app.start()
    except KeyboardInterrupt:
        log("Keyboard interrupt received")
    except Exception as e:
        log(f"[FATAL] Unhandled exception: {e}")
        log(traceback.format_exc())
    finally:
        app.shutdown()


if __name__ == '__main__':
    main()
