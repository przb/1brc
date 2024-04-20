use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use itertools::Itertools;

const FILENAME: &str = "measurements.txt";

fn main() {
    let file = fs::File::open(FILENAME).expect(format!("Unable to open measurements file (\"{FILENAME}\")").as_str());
    let metadata = fs::metadata(FILENAME).expect(format!("Unable to get metadata for measurements file (\"{FILENAME}\")").as_str());
    let reader = BufReader::new(file);

    // mapping of station name to min, sum, max, and count, respectively.
    let mut mappings: HashMap<String, (f64, f64, f64, usize)> = HashMap::new();

    for line in reader.lines() {
        let line = line.unwrap();
        if let Some((station, m)) = line.split_once(';') {
            let measurement = m.parse().expect("Unable to parse measurement");
            mappings.entry(station.into())
                .and_modify(|(min, sum, max, count)| {
                    *min = f64::min(*min, measurement);
                    *max = f64::max(*max, measurement);
                    *sum += measurement;
                    *count += 1;
                })
                .or_insert((measurement, measurement, measurement, 1));
        }
    }

    for (k, (min, sum, max, count)) in mappings.iter().sorted_by(|(k, _), (l, _)|k.cmp(l)) {
        let avg = sum / (*count as f64);

        println!("{k:100} | Avg: {avg:>5.1} | Min: {min:>5.1} | Max: {max:>5.1} | Count: {count:>10}");
    }
}
