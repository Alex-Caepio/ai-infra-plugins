fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match ai_infra_mod::dispatch(&args) {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
