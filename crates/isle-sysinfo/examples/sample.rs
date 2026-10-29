//! Prints two samples a second apart; with `--monitor`, a whole Monitor sample as JSON, then
//! the cost of fifty more at p50 and p95.

fn main() {
    if std::env::args().any(|a| a == "--monitor") {
        let mut sampler = isle_sysinfo::monitor::MonitorSampler::new();
        sampler.sample();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let full = sampler.sample();
        println!("{}", serde_json::to_string_pretty(&full).unwrap());
        let samples: Vec<_> = (0..100)
            .map(|_| {
                std::thread::sleep(std::time::Duration::from_millis(100));
                sampler.sample()
            })
            .collect();
        let at = |mut v: Vec<std::time::Duration>, q: usize| {
            v.sort();
            v[v.len() * q / 100]
        };
        let costs: Vec<_> = samples.iter().map(|s| s.cost).collect();
        eprintln!("{} processes; cost p50 {:?}, p95 {:?}", full.processes.len(), at(costs.clone(), 50), at(costs, 95));
        for (i, (name, _)) in full.parts.iter().enumerate() {
            let part: Vec<_> = samples.iter().map(|s| s.parts[i].1).collect();
            eprintln!("  {name}: p50 {:?}, p95 {:?}", at(part.clone(), 50), at(part, 95));
        }
        return;
    }
    let mut sampler = isle_sysinfo::Sampler::new(2);
    sampler.sample();
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("{:#?}", sampler.sample());
}
