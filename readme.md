
clang-format file required,
if not provided in neither the .json file nor as parameter, clang-format will still be executed
but without the --style=file parameter ?

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

This might seem a bit odd, especially if you're used to how `git` submodules work, but it is quite a common scenario in large projects. Such projects consist of multiple repositories that do not have a flat folder structure. Cloning repositories into other repositories is typically avoided, and therefore there exist no files in the root directory:

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

- The `.json` configuration file also needs to specify the `styleRoot` folder to which the tool will *temporarily* copy the format file.

> **Remark:** The `--style` parameter, if set, overrides the `styleFile` field.

> **Remark:** The tool will check whether a `.clang-format` file *with different content* already exists in `styleRoot` - and abort with an error if that is the case. Any user manually copying the `.clang-format` file to the root folder (e.g., to work with an editor supporting `clang-format`) would therefore be notified if their style file is outdated.


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

This tool does not determine the common denominator for all paths to place the `.clang-format` file there. Two reasons:

- First of all, no common root folder except the file system root might exist.
- Second, in case of failures or when aborting the tools you don't want `.clang-format` files flying around in your folder structure.
- Third, it would need special handling for *Scenario A*.