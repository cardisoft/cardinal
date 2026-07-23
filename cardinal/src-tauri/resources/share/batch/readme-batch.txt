macOS service menus contains Automator workflows to integrate Cardinal with Finder.

To install a workflow, double-click the .workflow item from this folder.
macOS installs it into the user's ~/Library/Services directory and manages Finder
Quick Action visibility through the standard system UI.

The "Cardinal, search in folder.workflow" item opens:

cardinal://search?scope=/path/to/folder

When Finder sends a file, the workflow uses the file's parent directory as the
scope. When Finder sends a folder, the workflow uses that folder as the scope.
