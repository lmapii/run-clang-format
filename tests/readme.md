
# Dependencies

The integration tests assume the following:

* `clang-format` is a valid command in your `$PATH`
* The same or a valid `clang-format` executable with version `10.0.1` exists as `<repo-root>/artifacts/clang/clang-format`

This is required to test most of the possible combinations and/or valid fields. The CI integrates this workflow in the test steps.
