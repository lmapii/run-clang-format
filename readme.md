# run_clang_format

[![Build status](https://github.com/lmapii/run_clang_format/workflows/ci/badge.svg)](https://github.com/lmapii/run_clang_format/actions)

CLI application for running an installed [`clang-format`](https://clang.llvm.org/docs/ClangFormat.html) and an existing `.clang-format` file on a set of files, specified using globs in a `.json` configuration file.

## Usage

The most basic execution of this CLI tool is as following:

```bash
$ run_clang_format path/to/format.json
```

Excecute `run_clang_format --help` for more details, or `run_clang_format schema` for a complete schema description of the configuration file.

The content of the configuration file is explained based on the following example:

```json
{
  "paths": [
    "../path/to/some/code/*.[ch]",
    "../../another/path/**/*.[ch]",
  ],
  "blacklist": [
    "FreeRTOSConfig.h",
    "*.pb.[ch]",
    "sdk_config.h"
  ],
  "styleFile": "./path/to/.clang-format",
  "styleRoot": "../",
  "command": "/Users/bugmenot/Downloads/clang-format"
}
```

- **paths** contains a list of paths or globs *relative to the directory of the JSON configuration file* that should be passed to `clang-format`. The glob syntax is described in the used [globset crate](https://docs.rs/globset/latest/globset/index.html#syntax) and is not repeated here (yet).

> *Remark:* Typically, globs do not contain relative path components. This tool, however, allows a relative path "prefix" before any glob that determines the root folder for a recursive search.

> *Remark:* Currently this tool skips all hidden files and folders (subject to change) to avoid walking through `.git` repositories.

- **blacklist** contains a list of paths, filenames or globs that should be excluded from formatting. Notice that a leading `**/` wildcard is not required, but added automatically for each item in the blacklist.

- **styleFile** is an optional parameter that specifies the path to the `.clang-format` file that should be used. See the below scenarios description for details.

> *Remark:* The command line parameter `--style` overrides the provided style file. The style file must either have the exact name `.clang-format` or use the file name extension `.clang-format`.

- **styleRoot** if a `.clang-format` file is defined, this tool will copy the style file to the provided directory before formatting the files, and will remove it on errors or onc all files have been formatted.

> *Remark:* If the execution is terminated early (using, e.g., `ctrl+c`), the tool will not be able to delete the temporary file.

- **command** optional field specifying the executable or command to use for formatting. By default the tool will try to use `clang-format`.

> **Remark:* The command line parameter `--command` overrides this *command* field.

## Planned/possible improvements

- [x] Cross-platform compilation with Github actions
- [ ] Don't simply exclude hidden files and folders?
- [ ] Testing (limited unit-tests but regression tests for all scenarios)
- [x] Concurrent execution of `clang-format` using a command line parameter `-j --jobs`
- [ ] Maybe switch to [indicatif](https://docs.rs/indicatif/latest/indicatif/) for reporting progress

## Use-Cases

### Scenario A: `.clang-format` exists and is placed correctly

Consider the following project, where the required `.clang-format` file is already placed in the root folder of the project. State of the art editors like [vscode](https://code.visualstudio.com) will allow developers to automatically format their files on save.

```
ProjectRoot
│
├── Some/Path
│   ├── header.h
│   └── source.c
│
├── Another/Path
│   └── <...>
│
├── format.json = {
│     paths = ["Some/Path/*.[ch]"]
│   }
│
└── .clang-format
```

Here this tool is of limited use, since files will rarely be left unformatted .. except for the odd developer who decides to use plain old `vi` or `emacs` without the corresponding plugins. Another use-case is to simply always execute the formatter upon pull-requests or to deal with problems like **line-endings** (can be enforced by `.clang-format`!).

#### How to deal with this?

Easy, neither the `--style` nor the corresponding entries in the `.json` configuration file need to be set. This tool will simply execute `clang-format` with the parameter `--style=file` on all files, `clang-format` itself will find the style file in the root folder of the project.


### Scenario B: A `.clang-format` file exists but is placed stored outside the root folder

This might seem a bit odd, especially if you're used to how `git` submodules work, but it is quite a common scenario in large projects. Such projects consist of multiple repositories that do not have a flat folder structure. Cloning repositories into other repositories is typically avoided, therefore there are no files in the root directory:

```
ProjectRoot
│
├── Some/Layer
│   ├── RepoA
│   ├── <...>
│   └── RepoX
│       ├── header.h
│       └── source.c
│
└── SettingRepos
    ├── format.json = {
    │     paths = ["Some/Path/*.[ch]"],
    │     styleFile = ".clang-format", [or --style]
    │     styleRoot = "../",
    │   }
    │
    ├── .clang-format
    └── <...>
```

With such a folder structure it would be necessary to copy the `.clang-format` file into the root directory - and update it in case of changes to the original file.

#### How to deal with this?

 - The `.clang-format` file can either be specified using the `--style` command line parameter, or is specified directly within the `.json` configuration file using the `styleFile` field.

- The `.json` configuration file also needs to specify the `styleRoot` folder to which the tool will *temporarily* copy the format file. Once the execution is complete, the temporary `.clang-format` file will be removed.

> *Remark:* The `--style` parameter, if set, overrides the `styleFile` field.

> *Remark:* The tool will check whether a `.clang-format` file *with different content* already exists in `styleRoot` - and abort with an error if that is the case. Any user manually copying the `.clang-format` file to the root folder (e.g., to work with an editor supporting `clang-format`) would therefore be notified if their style file is outdated.


### Scenario C: The `.clang-format` file is selected during runtime

This might be a theoretical one, this has been added since this is one of the first `rust` tools that I've written and I wanted to explore the command line parser.

So: The `.clang-format` file might be specified during runtime using the `--style` parameter, e.g., by a CI toolchain or another build script.

```
ProjectRoot
│
├── Some/Path
│   ├── header.h
│   └── source.c
│
└── format.json = {
      paths = ["Some/Path/*.[ch]"]
       styleRoot = "./",
    }
```

#### How to deal with this?

Since the tool assumes that no `.clang-format` file exists in the project, it needs to know where to place it. This neds to be specified within the `.json` configuration file, using the `styleRoot` field.

## What this tool doesn't do

### Multiple `.clang-format` files

In case `.clang-format` files exist in different folders, `.clang-format` will always use the first file that it finds when going backwards from the file to format. E.g., for `header.c` the file `ProjectRoot/Some/Layer/.clang-format` will be used.

```
ProjectRoot
│
├── Some/Layer
│   ├── .clang-format
│   │
│   ├── RepoA
│   ├── <...>
│   └── RepoX
│       ├── header.h
│       └── source.c
│
└── .clang-format
```

### I don't want to specify the root for `clang-format`

This tool does not determine the common denominator for all paths to place the `.clang-format` file there:

- No common root folder might exist, except the file system root.
- In case of failures or when aborting the tool, you don't want `.clang-format` files flying around in your folder structure.
- It would need special handling for *Scenario A*.
