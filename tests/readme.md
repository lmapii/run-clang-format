
# Dependencies

When trying to execute the integration tests locally, please check the following:

* `clang-format` is a valid command in your `$PATH`
* The same or a valid `clang-format` executable exists as `<repo-root>/artifacts/clang/clang-format[.exe]`
* Please check the [ci setup](../.github/setup/) for the `clang-format` version that is currently used for testing.

This setup is required to test most of the possible combinations and/or valid fields. The CI integrates this workflow in the test steps.
