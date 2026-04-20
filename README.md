
# 📎 Clipt

**Clipt** is a high-performance, privacy-focused clipboard history manager and AI assistant. It silently tracks your digital workflow, organizing every snippet you copy into a searchable, daily database, and allows you to chat with your history using the power of elite-scale AI.

![Clipt Banner](assets/banner.png)

## 🚀 Advanced Features

### 🧠 Intelligent UX
- **Massive Context AI Chat**: Engage in deep conversations with your clipboard history using **Qwen 3.5 (122B)** via **NVIDIA NIM**. 
- **256K Context Window**: Unlike standard clipboard managers, Clipt can process your entire day's history in a single prompt. Copy massive codebases, long documents, or thousands of snippets—the AI remembers it all.
- **Smart Window Management**: 
  - **Single Instance Lock**: Built-in socket-based protection ensures only one instance of Clipt runs at a time, preventing duplicate tray icons or process bloat.
  - **Close-to-Tray**: The app stays active in the system tray when closed, ensuring not a single clipboard event is missed.
- **Silent Clipboard Operations**: Custom PowerShell integration allows the app to write to your clipboard silently without flashing terminal windows.

### 🔒 Privacy & Organization
- **Daily History Vaults**: Your data is automatically organized into daily folders (`/Days/YYYY-MM-DD/`). Each day is powered by its own isolated **SQLite** database for maximum performance and portability.
- **Local-First Persistence**: All clips and configurations are stored securely in your system's data directory. Your history never leaves your machine.
- **Markdown Mastery**: Full support for rendering Markdown in AI responses, including syntax-highlighted code blocks, bold text, and structured lists.

### 📂 Data & Privacy
Clipt is built on the principle of data ownership. On the first launch, the app initializes a private workspace:
- **Windows:** `%APPDATA%/Clipt`

This folder contains your `Days/` archive (databases and metadata) and your `.env` configuration. Clipt does not use external cloud storage for your clips; your data remains strictly local.

### ⚡ Performance & Polish
- **Resource Efficient**: Built with Python 3.12 and `pywebview` to maintain a light footprint while providing a rich, modern UI.
- **Stealth Background Polishing**: Implements automation-controlled bypasses and high-quality Lanczos resampling for tray icons to ensure the app feels like a native part of the OS.
- **Neumorphic Design**: A premium, dark-mode interface with a custom silver-border aesthetic (#bdbec0) that matches modern productivity workflows.

---

## 🛠️ Command Line Arguments

Clipt supports a specialized startup mode for automation:

| Argument | Description |
| :--- | :--- |
| `--startup` | Launches Clipt directly to the system tray. No window will appear until you click the tray icon. Perfect for adding to your "Startup" folder. |

---

## 📥 Installation

### 1. Prerequisites
Ensure you have [Python 3.12](https://www.python.org/) installed.

### 2. Setup
```bash
# Clone the repository
git clone https://github.com/noahain/clipt

# Enter the project folder
cd clipt

# Install dependencies
pip install -r requirements.txt

# Run the app
python main.py
```

---

## 📦 Building the Executable (.EXE)

To package Clipt into a standalone Windows application that doesn't require Python:

1. **Run the build script:**
   ```bash
   python build_exe.py
   ```
2. **Locate your app:** The standalone binary will be generated in the `dist/` folder.
3. **Icons:** The build automatically bundles the high-res `icon.ico` and `icon.png` for proper Windows taskbar and system tray scaling.

---

## 🤖 Agentic Development (The Story)

Clipt is a product of **Human-AI Collaboration**, utilizing a multi-agent "Senior Developer" workflow.
- **Lead Architect:** Noahain (Product Design, Python Lifecycle, Logic Direction)
- **Primary Developer:** **Claude Code** (Powered by **Kimi K2.5**) - Implemented the core SQLite storage architecture, clipboard monitoring, and `pywebview` integration.
- **Technical Consultant:** **Gemini 1.5 Flash** - Provided architectural guidance, visual design strategies, and cross-process communication fixes.

---

## ⚖️ License & Disclaimer
Clipt is an independent productivity tool. AI responses are powered by NVIDIA NIM (Qwen 3.5 122B). Use at your own risk.

**License:** MIT 

Built with ❤️ and Artificial Intelligence.

