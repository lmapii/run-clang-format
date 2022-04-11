#!/bin/sh

brew install wget
brew install clang-format
brew link --overwrite clang-format

ver="14.0.0"
pkg="clang+llvm-$ver-x86_64-apple-darwin"

wget -O clang-$ver.tgz "https://github.com/llvm/llvm-project/releases/download/llvmorg-$ver/$pkg.tar.xz"
mkdir -p artifacts/clang
tar -xf clang-$ver.tgz $pkg/bin/clang-format
mv $pkg/bin/clang-format artifacts/clang
rm -rf $pkg
rm clang-$ver.tgz

ls -la artifacts/clang
artifacts/clang/clang-format --version
