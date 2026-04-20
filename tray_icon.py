"""
Tray Icon - System tray integration using pystray
"""

import pystray
from PIL import Image
from pathlib import Path

# Optional import for positioning custom menu
try:
    import pyautogui
    menu_pos = pyautogui.position
except ImportError:
    menu_pos = None


class TrayIcon:

    def __init__(self, app):
        self.app = app
        self.icon = None
        self._icon_path = Path(__file__).parent / "assets" / "icon.ico"

    def _load_icon_image(self):
        """Load and resize icon to proper tray size (64x64)"""
        try:
            if self._icon_path.exists():
                img = Image.open(self._icon_path)

                # ICO files have multiple sizes — find the largest frame
                if hasattr(img, 'n_frames') and img.n_frames > 1:
                    best_frame = 0
                    best_size = 0
                    for i in range(img.n_frames):
                        try:
                            img.seek(i)
                            size = img.size[0] * img.size[1]
                            if size > best_size:
                                best_size = size
                                best_frame = i
                        except:
                            pass
                    img.seek(best_frame)

                icon_image = img.convert("RGBA")
                # Resize to exactly 64x64 for consistent tray size
                icon_image = icon_image.resize((64, 64), Image.LANCZOS)
                return icon_image

        except Exception as e:
            print(f"Error loading tray icon: {e}")

        # Fallback: simple generated icon
        from PIL import ImageDraw
        image = Image.new('RGBA', (64, 64), (0, 0, 0, 0))
        dc = ImageDraw.Draw(image)
        dc.ellipse([4, 4, 60, 60], fill='#bdbec0')
        dc.ellipse([16, 16, 48, 48], fill='#161618')
        return image

    def _on_open(self, icon, item):
        """Menu: Open Clipt"""
        self.app.open_ui()

    def _on_pause(self, icon, item):
        """Menu: Pause/Resume watcher"""
        if self.app.watcher.paused:
            self.app.resume_watcher()
        else:
            self.app.pause_watcher()

    def _on_exit(self, icon, item):
        """Menu: Exit application"""
        icon.stop()
        self.app.shutdown()

    def _get_menu(self):
        """Build tray menu"""
        paused = self.app.watcher.paused if self.app.watcher else False
        return pystray.Menu(
            pystray.MenuItem("Open Clipt", self._on_open, default=True),
            pystray.MenuItem("Resume" if paused else "Pause", self._on_pause),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("Exit", self._on_exit)
        )

    def _on_click(self, icon):
        """Handle left-click - show main window"""
        icon.run_detached(lambda: self.app.open_ui())

    def _show_custom_menu(self):
        """Show custom webview menu at mouse position"""
        if menu_pos:
            x, y = menu_pos()
        else:
            x, y = 100, 100
        self.app.show_tray_menu(x, y)

    def run(self):
        """Start tray icon"""
        self.icon = pystray.Icon(
            'clipt',
            icon=self._load_icon_image(),
            title='Clipt',
            menu=self._get_menu(),
            on_clicked=self._on_click  # Left-click handler
        )
        self.icon.run()

    def stop(self):
        """Stop tray icon"""
        if self.icon:
            self.icon.stop()

    def update_menu(self, paused=None):
        """Update menu state"""
        if self.icon:
            self.icon.menu = self._get_menu()
