
@REM powershell -Command "Invoke-WebRequest https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.0/LLVM-14.0.0-win64.exe -OutFile llvm.exe"

choco install llvm --version 14.0.0 -y
dir "%SYSTEMDRIVE%\Program Files\LLVM\bin"

mkdir "artifacts\clang"
copy "%SYSTEMDRIVE%\Program Files\LLVM\bin\clang-format.exe" "artifacts\clang"

@REM artifacts\clang\clang-format.exe --version
