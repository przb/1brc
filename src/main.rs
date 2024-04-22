use std::collections::HashMap;
use std::{fs, thread};
use std::sync::{mpsc};
use std::sync::mpsc::Receiver;
use itertools::Itertools;
use memmap;

const FILENAME: &str = "measurements.txt";
const NUM_CHUNKS: usize = 16;

/// min, sum, max, and count, respectively
type ComputedMeasurements = (isize, isize, isize, usize);


fn process_line(chunk: &str, mappings: &mut HashMap<String, ComputedMeasurements>) {
    if let Some((station, m)) = chunk.split_once(';') {
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

fn process_file_chunks(rx: Receiver<String>) -> HashMap<String, ComputedMeasurements> {
    let mut mappings: HashMap<String, ComputedMeasurements> = HashMap::new();

    while let Ok(chunk) = rx.recv() {
        println!("received data");
        for line in chunk.lines() {
            // println!("processing line: {}", line);
            process_line(line, &mut mappings);
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
    let (mmap_tx, mmap_rx) = mpsc::channel();
    let file = fs::File::open(FILENAME).expect(format!("Unable to open measurements file (\"{FILENAME}\")").as_str());
    let metadata = file.metadata().expect("Unable to read file metadata");
    let chunk_size = metadata.len() as usize / NUM_CHUNKS;
    println!("Chunk size: {}", chunk_size);

    let mmap = unsafe { memmap::Mmap::map(&file).expect("Unable to map the file") };
    let bm = &mmap;
    thread::scope(|s| {
        for chunk_num in 0..NUM_CHUNKS {
            let tx = mmap_tx.clone();
            thread::Builder::new()
                .name(format!("mmap chunk division thread ({})", chunk_num))
                .spawn_scoped(s, move || {
                    let mut start = chunk_num * chunk_size;
                    let prev_start = start.checked_sub(chunk_size).unwrap_or_default();

                    let prev_map = bm.get(prev_start..start).unwrap_or_default();
                    let prev_new_line = prev_map.iter()
                        .rev()
                        .position(|&c| c == b'\n')
                        .unwrap_or_default();
                    start = start - prev_new_line;

                    let end = usize::min(start + chunk_size, bm.len());
                    let curr_map = bm.get(start..end).unwrap_or_default();
                    let last_new_line = curr_map.iter()
                        .rev()
                        .position(|&c| c == b'\n')
                        .unwrap_or_default();
                    let end = end - last_new_line;

                    tx.send((start, end))
                        .expect("Unable to send chunk to the channel");
                })
                .expect("Unable to spawn a new thread");
        }
        thread::Builder::new()
            .name("mmap chunk read dispatcher thread".into())
            .spawn_scoped(s, move || {
                let mut thread = 0;
                while let Ok((start, end)) = mmap_rx.recv() {
                    let thread_tx = tx.clone();
                    thread::Builder::new()
                        .name(format!("mmap chunk read thread {thread}"))
                        .spawn_scoped(s, move || {
                            let bytes = bm.get(start..end)
                                .expect("Unable to get the chunk");
                            let s = String::from_utf8_lossy(bytes);
                            thread_tx.send(s.into())
                                .expect("Unable to send chunk to the channel");
                        })
                        .expect("Unable to spawn a new thread");
                    thread += 1;
                }
            })
            .expect("Unable to spawn a new thread");
    });
}

fn main() {
    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name("reading thread".into())
        .spawn(|| { read_file(tx) })
        .expect("Unable to spawn a new thread");
    let out_thread = thread::Builder::new()
        .name("processing thread".into())
        .spawn(|| {
            let mappings = process_file_chunks(rx);
            print_mappings(mappings);
        })
        .expect("Unable to spawn a new thread");

    out_thread.join().expect("Unable to join the output thread");
}
