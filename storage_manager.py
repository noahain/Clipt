"""
Storage Manager - Handles daily folder organization and SQLite operations
"""

import os
import json
import sqlite3
from datetime import datetime
from pathlib import Path
from threading import Lock


class StorageManager:
    """Manages clipboard storage in daily organized folders"""

    def __init__(self, base_path=None):
        if base_path:
            self.base_path = Path(base_path)
        else:
            # Use Windows roaming folder: %APPDATA%/Clipt/Days
            import os
            appdata = os.environ.get('APPDATA')
            if not appdata:
                appdata = Path.home() / "AppData" / "Roaming"
            self.base_path = Path(appdata) / "Clipt" / "Days"
        self.base_path.mkdir(parents=True, exist_ok=True)
        self._locks = {}
        self._main_lock = Lock()

    def _get_day_folder(self, date_str):
        """Get or create the folder for a specific day"""
        folder_path = self.base_path / date_str
        folder_path.mkdir(parents=True, exist_ok=True)
        return folder_path

    def _get_db_path(self, date_str):
        """Get the database path for a specific day"""
        return self._get_day_folder(date_str) / "history.db"

    def _get_metadata_path(self, date_str):
        """Get the metadata file path for a specific day"""
        return self._get_day_folder(date_str) / "metadata.json"

    def _init_db(self, date_str):
        """Initialize SQLite database for a day"""
        db_path = self._get_db_path(date_str)

        with sqlite3.connect(str(db_path)) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT NOT NULL,
                    content TEXT NOT NULL
                )
            """)

            # Create index for faster queries
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_timestamp ON clips(timestamp)
            """)
            conn.commit()

    def _init_metadata(self, date_str):
        """Initialize metadata file for a day"""
        metadata_path = self._get_metadata_path(date_str)

        if not metadata_path.exists():
            metadata = {
                "date": date_str,
                "label": "",
                "created_at": datetime.now().isoformat()
            }
            with open(metadata_path, 'w') as f:
                json.dump(metadata, f, indent=2)

    def _get_lock(self, date_str):
        """Get a lock for a specific date"""
        with self._main_lock:
            if date_str not in self._locks:
                self._locks[date_str] = Lock()
            return self._locks[date_str]

    def save_clip(self, content, timestamp=None):
        """
        Save a clipboard entry to the current day's database
        Returns the ID of the inserted record
        """
        date_str = datetime.now().strftime('%Y-%m-%d')

        # Ensure database and metadata exist
        self._init_db(date_str)
        self._init_metadata(date_str)

        db_path = self._get_db_path(date_str)

        if timestamp is None:
            timestamp = datetime.now().isoformat()

        with self._get_lock(date_str):
            with sqlite3.connect(str(db_path)) as conn:
                cursor = conn.execute(
                    "INSERT INTO clips (timestamp, content) VALUES (?, ?)",
                    (timestamp, content)
                )
                conn.commit()
                return cursor.lastrowid

    def get_all_days(self):
        """Get list of all available days with metadata"""
        days = []

        if not self.base_path.exists():
            return days

        for folder in sorted(self.base_path.iterdir(), reverse=True):
            if folder.is_dir():
                date_str = folder.name
                metadata_path = folder / "metadata.json"

                if metadata_path.exists():
                    try:
                        with open(metadata_path, 'r') as f:
                            metadata = json.load(f)
                    except (json.JSONDecodeError, IOError):
                        metadata = {"date": date_str, "label": ""}
                else:
                    metadata = {"date": date_str, "label": ""}

                # Count clips for this day
                db_path = folder / "history.db"
                clip_count = 0
                if db_path.exists():
                    try:
                        with sqlite3.connect(str(db_path)) as conn:
                            cursor = conn.execute("SELECT COUNT(*) FROM clips")
                            clip_count = cursor.fetchone()[0]
                    except sqlite3.Error:
                        pass

                metadata['clip_count'] = clip_count
                metadata['folder_path'] = str(folder)
                days.append(metadata)

        return days

    def get_day_data(self, date_str):
        """Get all clips for a specific day"""
        db_path = self._get_db_path(date_str)
        metadata_path = self._get_metadata_path(date_str)

        if not db_path.exists():
            return {"clips": [], "metadata": {"date": date_str, "label": ""}}

        try:
            with sqlite3.connect(str(db_path)) as conn:
                # Enable row factory for dict-like results
                conn.row_factory = sqlite3.Row
                cursor = conn.execute(
                    "SELECT id, timestamp, content FROM clips ORDER BY timestamp ASC"
                )
                clips = [dict(row) for row in cursor.fetchall()]
        except sqlite3.Error as e:
            print(f"Error reading database for {date_str}: {e}")
            clips = []

        if metadata_path.exists():
            try:
                with open(metadata_path, 'r') as f:
                    metadata = json.load(f)
            except (json.JSONDecodeError, IOError):
                metadata = {"date": date_str, "label": ""}
        else:
            metadata = {"date": date_str, "label": ""}

        return {"clips": clips, "metadata": metadata}

    def update_day_label(self, date_str, label):
        """Update the label for a specific day"""
        metadata_path = self._get_metadata_path(date_str)

        # Ensure folder exists
        self._get_day_folder(date_str)
        self._init_metadata(date_str)

        try:
            with open(metadata_path, 'r') as f:
                metadata = json.load(f)

            metadata['label'] = label
            metadata['updated_at'] = datetime.now().isoformat()

            with open(metadata_path, 'w') as f:
                json.dump(metadata, f, indent=2)

            return True
        except (json.JSONDecodeError, IOError, KeyError) as e:
            print(f"Error updating metadata for {date_str}: {e}")
            return False
