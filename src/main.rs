#![warn(missing_debug_implementations, rust_2018_idioms)]

fn main() -> eyre::Result<()> {
    // println!(
    //     "Executing \n{} from \n{}\n",
    //     std::env::current_exe().unwrap().to_string_lossy(),
    //     std::env::current_dir().unwrap().to_string_lossy()
    // );

    let data = run_clang_format::cli::Builder::build().parse()?;
    run_clang_format::run(data)
}
