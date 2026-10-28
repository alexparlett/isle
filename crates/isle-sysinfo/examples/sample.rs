//! Prints two samples a second apart.

fn main() {
    let mut sampler = isle_sysinfo::Sampler::new(2);
    sampler.sample();
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("{:#?}", sampler.sample());
}
