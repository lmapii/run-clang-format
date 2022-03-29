mod cli;
mod lib;

fn main() -> eyre::Result<()> {
    // println!(
    //     "Executing \n{} from \n{}\n",
    //     std::env::current_exe().unwrap().to_string_lossy(),
    //     std::env::current_dir().unwrap().to_string_lossy()
    // );

    let matches = cli::build().get_matches();
    cli::setup(&matches);

    lib::run(matches)
}
