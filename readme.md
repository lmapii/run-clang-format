# run_clang_format <!-- omit in toc -->

[![Build status](https://github.com/lmapii/run_clang_format/workflows/ci/badge.svg)](https://github.com/lmapii/run_clang_format/actions)

CLI application for running an installed [`clang-format`](https://clang.llvm.org/docs/ClangFormat.html) and an existing `.clang-format` file on a set of files, specified using globs in a `.json` configuration file.

# Quickstart <!-- omit in toc -->

The minimal command for executing this is the following:

```bash
$ run_clang_format path/to/format.json
```

Execute `run_clang_format --help` for more details, or `run_clang_format schema` for a complete schema description of the configuration file.

**Hints for the impatient user:**

- Hidden paths and files are excluded unless the setting is changed [in the configuration file](#pre-filtering).
- This tool assumes that `clang-format` is installed and in your path. This can be changed in your [configuration file](#specifying-the--clang-format-command) or specified as a [command-line parameter](#specifying-an-alternative-style-file-and-command).
- Paths can be specified using [glob- or Unix-style path syntax](#glob--and-path-syntax).
- Formatting is [executed in parallel](#speeding-up-the-execution) if the `-j` option is specified.

# Contents <!-- omit in toc -->

- [The JSON configuration file](#the-json-configuration-file)
  - [Adding paths](#adding-paths)
  - [Glob- and path syntax](#glob--and-path-syntax)
  - [Pre-filtering](#pre-filtering)
  - [Post-filtering](#post-filtering)
  - [Specifying a `.clang-format` style file and a root directory](#specifying-a-clang-format-style-file-and-a-root-directory)
  - [Specifying the  `clang-format` command](#specifying-the--clang-format-command)
- [Command-line Parameters](#command-line-parameters)
  - [Verbosity and `--quiet`](#verbosity-and---quiet)
  - [Speeding up the execution](#speeding-up-the-execution)
  - [Specifying an alternative style file and command](#specifying-an-alternative-style-file-and-command)
- [Use-cases](#use-cases)
  - [A style file exists and is placed in the root folder](#a-style-file-exists-and-is-placed-in-the-root-folder)
  - [A style file exists but is placed stored outside the root folder](#a-style-file-exists-but-is-placed-stored-outside-the-root-folder)
  - [The style file is selected during runtime](#the-style-file-is-selected-during-runtime)
- [Possible pitfalls](#possible-pitfalls)
  - [Multiple `.clang-format` files](#multiple-clang-format-files)

# The JSON configuration file

The core of this CLI tool is a `.json` configuration file that specifies where all the files that should be formatted can be found. We'll be using a demo file, building it up step by step to explain the individual fields. The structure of the `.json` file is also documented in the `schema` sub-command (execute `run_clang_format schema`). To get started, we create an empty `.json` file that contains an empty object.

```json
{
}
```

## Adding paths

The only field that is really required in this configuration file is **`paths`**. This field contains paths or **globs**, relative to the parent directory of the configuration file. Consider the following folder structure:

```
ProjectRoot
│
├── Some
│   └── Path
│       ├── header.h
│       └── source.c
│
└── Settings
    ├── format.json
    └── <...>
```

In the configuration file `format.json`, the paths to the two files would need to be specified as follows:

```json
{
  "paths": [
    "../Some/Path/header.h",
    "../Some/Path/source.c",
  ],
}
```

> **Remark:** This tool is made for software developers, thus any user should know that paths by themselves can become fairly complex: Take links, throw in character encodings, you get the idea. So anyone using smileys or other surreal things in their paths can contribute to this repository in case of problems, but not all scenarios can or will be tested.

Clearly, no one wants to specify all paths manually, which is why this tool supports the use of Unix-style **globs**. The following patterns will all resolve to the same paths, but are just provided for reference:

```json
{
  "paths": [
    "../**/*.[ch]",
    "../Some/*/*.*",
  ],
}
```

Assuming you have `clang-format` installed and a `.clang-format` file in one of the parent directories of your sources, e.g., in *ParentRoot*, this is all you need::

```
$ run_clang_format path/to/format.json
```

Notice that the working directory of the tool is irrelevant since all paths are specified relative to the provided `format.json`. For now, this is all you need to know, we'll go into details about the supported scenarios later and will continue exploring the configuration options in the `.json` file.

## Glob- and path syntax

This tool uses the [globset](https://docs.rs/globset/latest/globset/index.html) rust crate to resolve globs. It therefore also relies on its [syntax](https://docs.rs/globset/latest/globset/index.html#syntax). We're borrowing the explanation here. When using globs, *standard Unix-style glob syntax* is supported:

- `?` matches any single character. It does not match path separators.
- `*` matches zero or more characters but does not match across directory boundaries, i.e., it does not match a path separator. You have to use `**` for that:
- `**` recursively matches directories and if used without a path separator it means "match everything".
- `{a,b}` matches `a` or `b` where `a` and `b` are arbitrary glob patterns. Nesting `{...}` is not supported.
- `[ab]` matches `a` or `b` where `a` and `b` are *characters*. Use `[!ab]` to match any character *except* for `a` and `b`.
- Metacharacters such as `*` and `?` can be escaped with the character class notation. e.g., `[*]` matches `*`.
- A backslash `\` will escape all metacharacters in a glob, but it must be specified as double backslash `\\` due to the fact that the glob is defined in a `.json` configuration file. If it precedes a non-meta character, then the slash is ignored.

For Windows paths, all globs are case insensitive.

> **Remark:** Due to the caveat that backslashes must be escaped in `.json` files, and that a backslash in a glob behaves differently depending on whether or not the following character is a metacharacter, it is highly recommended to use a forward slash `/` as path separator on **any** platform. On Windows it is possible to use `\\` as path separators, but only if it does not precede a metacharacter.

## Pre-filtering

By default, this tool will **exclude** all hidden files and folders from its search. This behaviour can be configured with the field **`filterPre`**. This field sets up a filter that is applied while recursively searching for files and therefore *before* matching files against the provided globs in the field `paths`. A typical pattern for such a filter is to exclude folders used by revision control systems, e.g., `.git` (or `.svn`) folders.

For this field, you can still use *globs*, but keep in mind that such a filter is applied on directories as well and thus if the filter matches then the directory will not even be searched, making it unnecessary to use, e.g., `**` after the name. The following example shows a pre-filter configured to exclude all files within the `.git` folder, and also excludes all hidden files and directories.

```json
{
  "paths": [
    "../**/*.[ch]",
    "../Some/*/*.*",
  ],
  "filterPre": ["**/.git", ".*"],
}
```

If no hidden folders should be skipped simply set this field to an empty list `[]`.

## Post-filtering

With the previous configuration file, we matched all files and folders except for hidden files. Sometimes, however, it is useful to apply a filter *after* matching all paths, e.g., to exclude specific filenames that occur multiple times, or to simplify the patterns in the field `paths`. This can be achieved with **`filterPost`**:

```json
{
  "paths": [
    "../**/*.[ch]",
    "../Some/*/*.*",
  ],
  "filterPre": ["**/.git", ".*"],
  "filterPost": ["FreeRTOS.h", "**/Hal*/**"],
}
```

In the above example, any `Hal*` folder within any of the paths will be filtered without having to create a complex glob for `paths`.

## Specifying a `.clang-format` style file and a root directory

If no `.clang-format` file is placed in the root directory of your project (assuming there is one), executing `run_clang_format` without any additional command-line parameters (explained below) would not produce the desired results - quite the opposite since `clang-format` checks any root folder until it might encounter a `.clang-format` file. Therefore the configuration file allows to specify the format file using the field **`styleFile`**, and the root common root directory of all paths using **`styleRoot`**:

```
ProjectRoot
│
├── Some
│   └── Path
│       ├── header.h
│       └── source.c
│
└── Settings
    ├── format.json
    ├── style.clang-format
    └── <...>
```

```json
{
  "paths": [
    "../**/*.[ch]",
    "../Some/*/*.*",
  ],
  "filterPre": ["**/.git", ".*"],
  "filterPost": ["FreeRTOS.h", "**/Hal*/**"],
  "styleFile": "./style.clang-format",
  "styleRoot": "../"
}
```

The name *or the extension* of the `styleFile` must be `.clang-format`. This allows you to store multiple `.clang-format` files in the same directory, e.g., `driver.clang-format` and `application.clang-format`.

When formatting the files, `run_clang_format` will:
- Copy the provided style file to the specified root directory (renaming it to `.clang-format`, if necessary),
- execute `clang-format` for all resolved paths,
- and finally remove the temporary file.

Only if you kill the execution of the tool (e.g., via CTRL+C) it won't be able to delete the temporary file.

> **Remark:** Specifying a root directory is necessary since it is not feasible to determine a common denominator for all paths. Also, killing the execution of the tool will prevent deleting the temporary file and therefore might clutter your workspace with format files, since adding new globs or paths might result in a different root directory.

> **Remark:** The tool will check whether a `.clang-format` file *with different content* already exists in `styleRoot` - and abort with an error if that is the case. If the contents match, the tool won't copy or delete any files and execute as if no `styleRoot` and `styleFile` were specified.

The `styleFile` configuration will be replaced by the **`--style`** command-line parameter, if provided.

## Specifying the  `clang-format` command

By default, the tool tries to use the command `clang-format` for formatting all resolved paths. If this command is not in your path, or if you use a different name for your executable (e.g., `clang-format-10`), then you need to specify the command or full path to the executable either via the command-line parameter `--command` or using the `command` field in your configuration file:

```json
{
  "paths": [
    "../**/*.[ch]",
    "../Some/*/*.*",
  ],
  "filterPre": ["**/.git", ".*"],
  "filterPost": ["FreeRTOS.h", "**/Hal*/**"],
  "styleFile": "./style.clang-format",
  "styleRoot": "../",
  "command": "/path/to/clang-format"
}
```

Similar to the `styleFile` field, this configuration will be replaced by the **`--command`** command-line parameter, if provided.

> **Notice:** It is important that your style file is compatible with the version of `clang-format` that you are using. This is the main reason why `clang-format` is not installed with this tool.

# Command-line Parameters

All available command-line parameters should be sufficiently described by the tool itself, when providing any of the options `-h, --help, help`. Also, the JSON schema of the configuration file can be displayed by using the `schema` subcommand. This JSON schema also contains descriptions for each of the options described above:

```
$ run_clang_format --help
$ run_clang_format help
$ run_clang_format schema
```

In the following, the most important options are described briefly.

## Verbosity and `--quiet`

The verbosity is best configured by using the `-v` option:

* `-v` is the default option; the tool will provide a "pretty-print" output complete with progress bar (implemented by the rust crate [indicatif](https://github.com/mitsuhiko/indicatif)).

> The "pretty" output is only available for the `-v` log level, for any other log level the tool will switch to a debug-style output. This kind of output is not optimized for being redirected to a file since the progress bar will rewrite previous lines. Use the `-vv` debug option instead.

* `-vv` switches to the log level "debug", providing timestamps and a purely sequential output: No lines are being overwritten, and each message is logged to a new line.

* `-vvv` and above switch to the log level "trace", which can contain even more (probably irrelevant) messages. This is intended mainly for debugging the tool in case you find issues.

To turn off any kind of output except for error messages, use the `--quiet` option. This overwrites the `--verbosity` level.

## Speeding up the execution

By default, the tool will process each resolved path one by one. This can be rather slow for large projects. The command-line option `-j, --jobs` allows specifying the number of jobs that should be used for formatting.

* If specified without a value, e.g., `run_clang_format format.json -j`, then all available logical cores will be used for formatting.
* If specified *with* a value, e.g., `run_clang_format format.json -j 3`, then the tool will only spawn as many jobs as specified.

> **Remark:** On slower machines, when executed with normal log level, the progress bar might flicker since the terminal might not be able to re-draw the new line fast enough. Currently, there's no way around this.

## Specifying an alternative style file and command

The command-line options `--style` and `--command` allow specifying a `.clang-format` file and the command to use for executing `clang-format`. Please refer to the corresponding section in the previous description of the `.json` configuration file (fields `styleFile` and `command`).

> **Remark:** Specifying `--style` requires the field `styleRoot` to be configured.

# Use-cases

The following scenarios demonstrate the use-cases that have been considered during the development of this tool.

> For the sake of simplicity, for all scenarios the `--command` option and the `command` field in the configuration file have been omitted.


## A style file exists and is placed in the root folder

Consider the following project, where the required `.clang-format` file is already placed in the root folder of the project. State-of-the-art editors like [vscode](https://code.visualstudio.com) will allow developers to automatically format their files on save.

```
ProjectRoot
│
├── Some
│   └── Path
│       ├── header.h
│       └── source.c
│
├── Another/Path
│   └── <...>
│
├── format.json
└── .clang-format
```

The following configuration file would allow to format all files in the project:

```json
{
  "paths": [
    "./**/*.[ch]",
  ],
}
```

Here this tool may seem to be of limited use since files will rarely be left unformatted. However, it helps to have a tool in place that formats all files on request, e.g., in the CI or on pull requests.

## A style file exists but is placed stored outside the root folder

This might seem a bit odd, especially if you're used to how `git` submodules work, but it is quite a common scenario in large projects.

Such projects consist of multiple repositories that do not have a flat folder structure. Cloning repositories into other repositories is typically avoided, therefore there are no files in the root directory:

```
ProjectRoot
│
├── Layers
│   ├── RepoA
│   ├── <...>
│   └── RepoX
│       ├── header.h
│       └── source.c
│
└── SettingsRepo
    ├── format.json
    ├── .clang-format
    └── <...>
```

Without a wrapper script, with such a folder structure it would be necessary to copy the `.clang-format` file into the root directory - and update it in case of changes to the original file. The following configuration file could be used instead:

```json
{
  "paths": [
    "./Layers/**/*.[ch]",
  ],
  "styleFile": ".clang-format",
  "styleRoot": "../"
}
```

> **Remark:** Since the tool checks whether a `.clang-format` file *with different content* already exists in `styleRoot`, any user manually copying the `.clang-format` file to the root folder (e.g., to work with an editor supporting `clang-format`) would be notified if their style file is outdated (the contents no longer match).

## The style file is selected during runtime

This might be a theoretical one, but it is still possible. The `.clang-format` file might be specified during runtime using the `--style` parameter, e.g., by a CI toolchain or another build script.

```
ProjectRoot
│
├── Some
│   └── Path
│       ├── header.h
│       └── source.c
│
└── Settings
    ├── format.json
    ├── .clang-format
    └── <...>
```

In this case, it is only necessary to specify the `paths` and the `styleRoot` in your configuration file:

```json
{
  "paths": [
    "./**/*.[ch]",
  ],
  "styleRoot": "../"
}
```

# Possible pitfalls

## Multiple `.clang-format` files

In case other `.clang-format` files exist in different folders, `.clang-format` will always use the first file that it finds when going back from the file to format. E.g., for `header.c` the file `ProjectRoot/Some/Layer/.clang-format` will be used.

```
ProjectRoot
│
├── Some
│   ├── .clang-format
│   └── Path
│       ├── header.h
│       └── source.c
│
└── .clang-format
```

When executing the tool with the following configuration, the files in `Some/Path` will be formatted using `Some/.clang-format` and **not** with the configured style file, since this tool does not scan any paths for existing `.clang-format` files.

```json
{
  "paths": [
    "./Layers/**/*.[ch]",
  ],
  "styleFile": ".clang-format",
  "styleRoot": "../"
}
```

