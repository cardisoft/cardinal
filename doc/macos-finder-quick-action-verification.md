# macOS Finder Quick Action Verification

This document verifies that Cardinal's macOS packaging exposes and installs the
Finder Quick Action through both the DMG and Homebrew Cask paths.

## Scope

The Finder integration is an Automator Quick Action, not a Finder Sync
extension. The workflow opens Cardinal through the public URL scheme:

```text
cardinal://search?scope=/path/to/folder&q=keyword
```

The Finder workflow uses `scope` only. When Finder sends a folder, the folder is
used as the search scope. When Finder sends a file, the file's parent directory
is used as the search scope.

## Build the DMG

From the repository root:

```bash
cd cardinal
npm run package:macos-dmg
```

Expected artifact:

```text
cardinal/src-tauri/target/release/Cardinal_0.1.23_aarch64.dmg
```

The generated DMG root should contain:

```text
Cardinal.app
Applications
macOS service menus/
README - Finder Quick Action.txt
```

## Reset Local State

Before testing either install path, remove previous app and workflow installs:

```bash
brew uninstall --cask zc/cardinal-local/cardinal-local --force || true
brew uninstall --cask cardinal-search --force || true
rm -rf /Applications/Cardinal.app
rm -rf "$HOME/Library/Services/Cardinal, search in folder.workflow"
killall Finder
```

## DMG Install Path

Open the generated DMG:

```bash
open cardinal/src-tauri/target/release/Cardinal_0.1.23_aarch64.dmg
```

In Finder:

1. Drag `Cardinal.app` to `Applications`.
2. Open `macOS service menus`.
3. Double-click `Cardinal, search in folder.workflow`.
4. Confirm the macOS workflow installation prompt.

Verify the installed app and workflow:

```bash
test -d /Applications/Cardinal.app
test -d "$HOME/Library/Services/Cardinal, search in folder.workflow"
plutil -lint "$HOME/Library/Services/Cardinal, search in folder.workflow/Contents/Info.plist"
```

Verify the URL scheme:

```bash
open -a Cardinal
open 'cardinal://search?scope=%2Ftmp&q=report'
```

Refresh Finder, then verify the context menu manually:

```bash
killall Finder
```

Right-click a file or folder in Finder. `Search in Cardinal` should appear under
Quick Actions or Services.

## Local Homebrew Cask Path

Create a local tap if it does not already exist:

```bash
brew tap-new zc/cardinal-local || true
mkdir -p "$(brew --repository zc/cardinal-local)/Casks"
```

Create the local cask:

```bash
cat > "$(brew --repository zc/cardinal-local)/Casks/cardinal-local.rb" <<'RUBY'
cask "cardinal-local" do
  version "0.1.23"
  sha256 :no_check

  url "file:///Users/zc/codespace/rust/cardinal/cardinal/src-tauri/target/release/Cardinal_0.1.23_aarch64.dmg"
  name "Cardinal"
  desc "Fast file search for macOS"
  homepage "https://github.com/cardisoft/cardinal"

  app "Cardinal.app"
  service "macOS service menus/Cardinal, search in folder.workflow"
end
RUBY
```

Check that Homebrew recognizes both artifacts:

```bash
brew info --cask zc/cardinal-local/cardinal-local
```

Expected artifact output:

```text
Cardinal.app (App)
macOS service menus/Cardinal, search in folder.workflow (Service)
```

Install through the local cask:

```bash
brew install --cask zc/cardinal-local/cardinal-local
```

Verify the installed app and workflow:

```bash
test -d /Applications/Cardinal.app
test -d "$HOME/Library/Services/Cardinal, search in folder.workflow"
plutil -lint "$HOME/Library/Services/Cardinal, search in folder.workflow/Contents/Info.plist"
```

Verify the URL scheme:

```bash
open -a Cardinal
open 'cardinal://search?scope=%2Ftmp&q=report'
```

Refresh Finder, then verify the context menu manually:

```bash
killall Finder
```

Right-click a file or folder in Finder. `Search in Cardinal` should appear under
Quick Actions or Services.

## Homebrew Uninstall Verification

Uninstall the local cask:

```bash
brew uninstall --cask zc/cardinal-local/cardinal-local --force
```

Verify Homebrew removed both artifacts:

```bash
test ! -d /Applications/Cardinal.app
test ! -d "$HOME/Library/Services/Cardinal, search in folder.workflow"
```

## Troubleshooting

If the workflow exists but Finder does not show the Quick Action:

```bash
killall Finder
```

If it still does not appear, check macOS Services settings:

```text
System Settings -> Keyboard -> Keyboard Shortcuts -> Services
```

The item should be named `Search in Cardinal`.

If `cardinal://` does not open Cardinal, launch the app once and retry:

```bash
open -a Cardinal
open 'cardinal://search?scope=%2Ftmp&q=report'
```
