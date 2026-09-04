# HashRename

Cross-platform file hash deduplication and sequential renaming tool.

## Features

- **MD5-based deduplication**: Content-based duplicate detection using streaming MD5 hashing
- **Two-phase verification**: MD5 match followed by byte-for-byte content verification to prevent hash collisions
- **Smart retention**: Automatically keeps the natural-sorted first file from each duplicate group
- **Safe trash**: Moves duplicates to system recycle bin/trash (never permanent delete)
- **Sequential renaming**: Renames remaining files as zero-padded numbers (001.xxx, 002.xxx, etc.)
- **Two-phase rename**: Uses temporary files to prevent overwriting existing files
- **Cross-platform**: Windows, macOS, Linux with shared core logic
- **CLI and GUI**: Command-line interface and Tauri-based GUI

## Supported Platforms

| Platform | Context Menu | Trash Integration |
|----------|--------------|-------------------|
| Windows  | Explorer right-click | Recycle Bin |
| macOS    | Finder Services | Trash |
| Linux    | Nautilus scripts | XDG Trash |

## Installation

### Windows

Download `HashRename_Setup.exe` from [Releases](https://github.com/jerry/HashRename/releases) and run the installer.

### macOS

Download `HashRename.dmg` from [Releases](https://github.com/jerry/HashRename/releases), open the DMG, and drag the app to Applications.

### Linux

Download `HashRename.AppImage` from [Releases](https://github.com/jerry/HashRename/releases):

```bash
chmod +x HashRename.AppImage
./HashRename.AppImage
```

Or install the `.deb` package:

```bash
sudo dpkg -i hashrename_0.1.0_amd64.deb
```

## Right-Click Menu Usage

### Install Context Menu

**Windows:**
```bash
HashRename.exe --install-context-menu
```

**macOS:**
```bash
./HashRename --install-context-menu
```

**Linux:**
```bash
./hashrename --install-context-menu
```

After installation, right-click any folder in your file manager and select "Hash 去重并重命名" (Hash Dedup & Rename).

### Uninstall Context Menu

```bash
HashRename --uninstall-context-menu
```

## CLI Usage

```bash
# Process a directory
hashrename /path/to/directory

# Verbose output
hashrename --verbose /path/to/directory

# Show help
hashrename --help

# Show version
hashrename --version

# Install context menu
hashrename --install-context-menu

# Uninstall context menu
hashrename --uninstall-context-menu
```

## Working Principles

### Deduplication Rules

1. **Size pre-filter**: Only files with identical sizes are compared
2. **MD5 hashing**: Streaming MD5 computation on size-matched groups
3. **Content verification**: Byte-for-byte comparison when MD5s match
4. **Non-recursive**: Only processes files in the selected directory (no subdirectories)

### Retention Rules

When multiple files are identical:
- Files are sorted by natural order (e.g., IMG_1.jpg before IMG_2.jpg before IMG_10.jpg)
- The first file in natural sort order is kept
- All others are moved to trash

### Rename Rules

- Files are renamed to `{number}.{extension}` (e.g., 001.jpg, 002.png)
- Minimum 3 digits, auto-expands for >999 files
- Extension case is preserved
- Natural sort order determines numbering
- Two-phase rename prevents overwriting existing files

### Trash Rules

- **Never permanent delete**: Duplicates always go to system trash/recycle bin
- **Safe failure**: If trash is unavailable, operation fails gracefully
- Platform-appropriate trash mechanism:
  - Windows: Recycle Bin
  - macOS: Trash
  - Linux: XDG Trash (~/.local/share/Trash)

## Development

### Prerequisites

- Rust (latest stable)
- Node.js (v20+)
- npm

### Setup

```bash
# Clone repository
git clone https://github.com/jerry/HashRename.git
cd HashRename

# Install frontend dependencies
npm install

# Run development build
npx tauri dev

# Run tests
cd src-tauri
cargo test
```

### Building

```bash
# Build for production
npx tauri build

# Build CLI only
cd src-tauri
cargo build --release --bin hashrename
```

## Testing

```bash
cd src-tauri
cargo test
```

Tests cover:
- MD5 hashing (empty, small, binary, Unicode files)
- Duplicate detection (identical files, size-only matches)
- Natural sorting (numbers, Chinese, mixed)
- Renaming (basic, digit width, extension preservation)
- Error handling (missing directories, permission errors)
- Trash operations (XDG compliance)
- Concurrent execution prevention

## GitHub Actions

### CI (ci.yml)

Runs on push/PR to main:
- Compiles Rust code
- Runs all tests
- Validates build

### Release (release.yml)

Triggered by version tags (`v*`):
- Builds for Windows, macOS, Linux
- Creates GitHub Release with artifacts

## Common Questions

**Q: Will this delete my files?**
A: No. Duplicate files are moved to system trash/recycle bin, never permanently deleted.

**Q: What if MD5 collides?**
A: After MD5 matching, byte-for-byte content verification prevents false positives.

**Q: Can I undo the renaming?**
A: The original filenames are not preserved after processing. Keep backups of important directories.

**Q: Does it process subdirectories?**
A: No. Only the selected directory's immediate files are processed.

**Q: What about file permissions?**
A: Files that cannot be read are skipped with an error recorded.

## Platform Verification Status

| Platform | Build | CLI Test | GUI Test | Context Menu | Trash |
|----------|-------|----------|----------|-------------|-------|
| Linux (Ubuntu 24.04) | ✅ Local build | ✅ Verified | ✅ Verified | ✅ Nautilus script installed | ✅ XDG Trash verified |
| Windows (GitHub Actions) | ✅ CI build | CI only | CI only | CI only | CI only |
| macOS (GitHub Actions) | ✅ CI build | CI only | CI only | CI only | CI only |

**Note:** Linux was fully tested on a physical machine. Windows and macOS builds are generated by GitHub Actions CI but have not been tested on physical hardware. Context menu integration on Windows and macOS should be verified after first release.

## Known Limitations

- **No undo for renaming**: Original filenames are lost after processing
- **Linux file manager support**: Currently only Nautilus; Dolphin/Nemo/Thunar may require manual script installation
- **macOS Finder integration**: Uses Automator Services, may require logout/login to activate
- **Windows long paths**: Requires Windows 10 1607+ with long path support enabled
- **Hash algorithm**: Currently MD5 only; SHA-256 support planned for future versions

## License

MIT License - see [LICENSE](LICENSE) for details.
