#![forbid(unsafe_code)]

fn main() {
    let _entrypoint = wr_client::init_entrypoint();
    let _runtime = wr_client::target_runtime();

    match wr_client::run(std::env::args().skip(1)) {
        Ok(Some(summary)) => {
            println!("{}", summary.summary_line());
        }
        Ok(None) => println!("{}", wr_client::help_text()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
