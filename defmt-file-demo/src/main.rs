fn main() {
    defmt_file::Logger::init("defmt.log").unwrap();
    println!("Hello, world!");
    defmt::println!("Hello, world!");
}
