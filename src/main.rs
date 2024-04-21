use std::collections::HashMap;
use std::{fs, thread};
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use itertools::Itertools;

const FILENAME: &str = "measurements.txt";
const CHUNK_SIZE: usize = 1_000_000;

/// min, sum, max, and count, respectively
type ComputedMeasurements = (isize, isize, isize, usize);

fn process_lines(rx: Receiver<String>) -> HashMap<String, ComputedMeasurements> {
    let mut mappings: HashMap<String, ComputedMeasurements> = HashMap::new();

    while let Ok(line) = rx.recv() {
        if let Some((station, m)) = line.split_once(';') {
            let (l, r) = m.split_once('.').expect("Did not find a decimal in the measurement");
            let int = l.parse::<isize>().expect("unable to parse the integer part of the measurement");
            let dec = r.parse::<isize>().expect("unable to parse the decimal part of the measurement");
            let measurement = (int * 10) + dec;
            mappings.entry(station.into())
                .and_modify(|(min, sum, max, count)| {
                    *min = isize::min(*min, measurement);
                    *max = isize::max(*max, measurement);
                    *sum += measurement;
                    *count += 1;
                })
                .or_insert((measurement, measurement, measurement, 1));
        }
    }
    mappings
}

fn print_mappings(mappings: HashMap<String, ComputedMeasurements>) {
    for (k, (min, sum, max, count)) in mappings.iter().sorted_by(|(k, _), (l, _)| k.cmp(l)) {
        // dividing by 10 to convert the fixed point to a floating point
        let avg = (*sum as f64 / 10.0) / (*count as f64);
        let min = *min as f64 / 10.0;
        let max = *max as f64 / 10.0;
        println!("{k:100} | Avg: {avg:>5.1} | Min: {min:>5.1} | Max: {max:>5.1} | Count: {count:>10}");
    }
}

fn read_file(tx: mpsc::Sender<String>) {
    let file = fs::File::open(FILENAME).expect(format!("Unable to open measurements file (\"{FILENAME}\")").as_str());
    let reader = BufReader::new(file);

    thread::spawn(move || {
        reader.lines().for_each(|line| {
            let line = line.expect("Unable to read line");
            tx.send(line).expect("Unable to send line to the channel");
        });
    });
}

fn main() {
    let (tx, rx) = mpsc::channel();
    read_file(tx);
    let mappings = process_lines(rx);
    print_mappings(mappings);
}
