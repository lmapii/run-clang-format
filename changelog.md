
# 1.4.0

- Renamed project to `run-clang-format`

# 1.3.0

- Added command-line parameter `--check`, supported for `clang-format` version 10 or higher. This runs `clang-format --dry-run -WError` on all resolved paths to check whether the formatting matches the specified file.

# 1.2.0

- The tool is now subject to some very basic integration tests, but executed on Windows, macos and Linux (Ubuntu).
- If executed on Windows it is now possible to omit the `.exe` extension for the CLI parameter `--command` and the `<JSON>` field `command` respectively. This allows to use configuration files on all operating systems.
- The fields `styleFile` and `command` in the configuration file are now only evaluated if no override is provided by the command line parameter `--style` or `--command`.

# 1.1.0

- Support for relative paths for the field 'command'.

# 1.0.0

- Breaking change in the `JSON` schema: The field `blacklist` has been renamed to `filterPost`.
- Added the `filterPre` field in the configuration file, allowing to specify custom path filters applied while expanding the globs.
- First proper content for the [readme](./readme.md).

# 0.3.2

- Fixes a bug in `0.3.1` hiding the error messages produced by `clang-format`.

# 0.3.1

- Adds the `--jobs` parameter for parallel execution.

# 0.0.0 to 0.3.0

- These tags have been created for testing the tool on various platforms.
