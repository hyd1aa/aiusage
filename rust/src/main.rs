fn main() {
    std::process::exit(aiusage::cli::main(std::env::args().skip(1).collect()));
}
