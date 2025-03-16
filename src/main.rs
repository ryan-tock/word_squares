use crate::trie::TrieNode;
use crossbeam_channel::{Sender, unbounded};
use itertools::Itertools;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{fs, io, thread};

mod trie;
const WORDS: &str = include_str!("words.txt");

const ROWS: usize = 4;
const COLUMNS: usize = 4;
const PRINT_SOLUTIONS: bool = false;
const THREADS: usize = 6;
const NUM_QUEUE_LETTERS: usize = 2;
enum Message {
    Solution([usize; COLUMNS]),
    File(String),
}
fn main() {
    let start = Instant::now();

    let words_row = WORDS.lines().filter(|w| w.len() == COLUMNS).collect_vec();
    let words_col = WORDS.lines().filter(|w| w.len() == ROWS).collect_vec();
    let (words_row, words_col) = if ROWS == COLUMNS {
        drop(words_col);
        (&words_row, &words_row)
    } else {
        (&words_row, &words_col)
    };
    if ROWS > 12 {
        println!("Cannot generate word grids with more than 12 rows");
        return;
    }
    if ROWS > COLUMNS {
        println!("Cannot generate word grids with more rows than columns");
        return;
    }
    if words_row.len() == 0 {
        println!("No words exist of size {COLUMNS}");
        return;
    }
    if words_col.len() == 0 {
        println!("No words exist of size {ROWS}");
        return;
    }

    // println!("{:?}", std::mem::size_of::<TrieNode>());

    let trie_row = TrieNode::from_words(words_row);
    let trie_col = TrieNode::from_words(words_col);

    let (trie_row, trie_col) = if ROWS == COLUMNS {
        drop(trie_col);
        (&trie_row, &trie_row)
    } else {
        (&trie_row, &trie_col)
    };

    let positions = {
        let mut positions = Vec::new();
        let mut position = (0u8, 0u8);
        for _ in 0..(ROWS * COLUMNS) {
            positions.push(position);
            position = next_pos(position);
        }
        positions
    };

    let queue = generate_queue(
        [0; COLUMNS],
        std::array::from_fn(|_| trie_row),
        std::array::from_fn(|_| trie_col),
        &positions,
        0,
        NUM_QUEUE_LETTERS,
    );
    let filenames = queue
        .iter()
        .map(|(words, _, _)| get_filename(words, &positions))
        .collect_vec();
    let queue = queue
        .into_iter()
        .sorted_by(|(_, row_1, _), (_, row_2, _)| row_2[0].leaf_nodes().cmp(&row_1[0].leaf_nodes()))
        .collect_vec();
    let queue = Arc::new(Mutex::new(queue.into_iter()));

    let num_solutions = Arc::new(AtomicUsize::new(0));

    thread::scope(|scope| {
        for _ in 0..THREADS {
            let (s, r) = unbounded::<Message>();
            let num_solutions = Arc::clone(&num_solutions);
            let queue = Arc::clone(&queue);
            let positions = &positions;

            scope.spawn(move || {
                loop {
                    let queue = Arc::clone(&queue);
                    let task = queue.lock().unwrap().next();
                    drop(queue);

                    match task {
                        Some(_) => {}
                        None => break,
                    }

                    let (words, nodes_row, nodes_col) = task.unwrap();

                    if PRINT_SOLUTIONS {
                        s.send(Message::File(get_filename(&words, positions)))
                            .unwrap();
                    }

                    let found_solutions = find_grids(
                        words,
                        nodes_row,
                        nodes_col,
                        positions,
                        NUM_QUEUE_LETTERS as u8,
                        &s,
                    );

                    num_solutions.fetch_add(found_solutions, Ordering::Relaxed);
                }
            });
            if PRINT_SOLUTIONS {
                scope.spawn(move || {
                    let mut output_file = File::open("src/main.rs").unwrap();
                    let mut writer = BufWriter::new(output_file);
                    for message in r.iter() {
                        match message {
                            Message::Solution(words) => {
                                writer.write(to_string(&words).as_bytes()).unwrap();
                                writer.write("\n\n".as_bytes()).unwrap();
                            }
                            Message::File(prefix) => {
                                output_file = File::create(prefix).unwrap();
                                writer = BufWriter::new(output_file);
                            }
                        }
                    }
                });
            }
        }
    });

    let output_name = format!("{}x{}.txt", ROWS, COLUMNS);
    if PRINT_SOLUTIONS && NUM_QUEUE_LETTERS > 0 {
        let mut output_file = File::create(&output_name).unwrap();

        for filename in filenames {
            let mut file = File::open(&filename).unwrap();
            io::copy(&mut file, &mut output_file).unwrap();
            fs::remove_file(filename).unwrap();
        }
    }
    if PRINT_SOLUTIONS && num_solutions.load(Ordering::Relaxed) == 0 {
        fs::remove_file(&output_name).unwrap();
    }

    println!(
        "Found {} {ROWS}x{COLUMNS} word grids in {:?}",
        num_solutions.load(Ordering::Relaxed),
        start.elapsed()
    ); // 2023763375 4x4 word grids in 58.468247709s
}

fn find_grids(
    mut words: [usize; COLUMNS],
    mut nodes_row: [&TrieNode; ROWS],
    mut nodes_col: [&TrieNode; COLUMNS],
    pos_vector: &Vec<(u8, u8)>,
    pos_i: u8,
    sender: &Sender<Message>,
) -> usize {
    if pos_i >= pos_vector.len() as u8 {
        if PRINT_SOLUTIONS {
            sender.send(Message::Solution(words)).unwrap();
        }
        return 1;
    }
    let (x, y) = pos_vector[pos_i as usize];

    if !nodes_row[y as usize].has_children || !nodes_col[x as usize].has_children {
        return 0;
    }

    let mut total = 0;
    for i in 0..26 {
        match nodes_row[y as usize].children[i] {
            Some(_) => {}
            None => {
                continue;
            }
        }
        match nodes_col[x as usize].children[i] {
            Some(_) => {}
            None => {
                continue;
            }
        }

        let old_node_row = nodes_row[y as usize];
        let old_node_col = nodes_col[x as usize];
        let old_word = words[x as usize];

        nodes_row[y as usize] = nodes_row[y as usize].children[i].as_ref().unwrap();
        nodes_col[x as usize] = nodes_col[x as usize].children[i].as_ref().unwrap();
        words[x as usize] += i << (y * 5);

        total += find_grids(words, nodes_row, nodes_col, pos_vector, pos_i + 1, sender);

        nodes_row[y as usize] = old_node_row;
        nodes_col[x as usize] = old_node_col;
        words[x as usize] = old_word;
    }

    total
}

fn generate_queue<'a>(
    mut words: [usize; COLUMNS],
    mut nodes_row: [&'a TrieNode; ROWS],
    mut nodes_col: [&'a TrieNode; COLUMNS],
    pos_vector: &Vec<(u8, u8)>,
    pos_i: u8,
    queue_letters: usize,
) -> Vec<(
    [usize; COLUMNS],
    [&'a TrieNode; ROWS],
    [&'a TrieNode; COLUMNS],
)> {
    let (x, y) = pos_vector[pos_i as usize];
    if queue_letters == 0 {
        return vec![(words, nodes_row, nodes_col)];
    }
    if !nodes_row[y as usize].has_children || !nodes_col[x as usize].has_children {
        return vec![];
    }

    let mut queue = Vec::new();
    for i in 0..26 {
        match nodes_row[y as usize].children[i] {
            Some(_) => {}
            None => {
                continue;
            }
        }
        match nodes_col[x as usize].children[i] {
            Some(_) => {}
            None => {
                continue;
            }
        }

        let old_node_row = nodes_row[y as usize];
        let old_node_col = nodes_col[x as usize];
        let old_word = words[x as usize];

        nodes_row[y as usize] = nodes_row[y as usize].children[i].as_ref().unwrap();
        nodes_col[x as usize] = nodes_col[x as usize].children[i].as_ref().unwrap();
        words[x as usize] += i << (y * 5);

        queue.extend(generate_queue(
            words,
            nodes_row,
            nodes_col,
            pos_vector,
            pos_i + 1,
            queue_letters - 1,
        ));

        nodes_row[y as usize] = old_node_row;
        nodes_col[x as usize] = old_node_col;
        words[x as usize] = old_word;
    }

    queue
}
fn next_pos(pos: (u8, u8)) -> (u8, u8) {
    let (x, y) = pos;
    if x >= y + 1 {
        if y + 1 == ROWS as u8 {
            (x + 1, 0)
        } else {
            if x == y + 1 { (0, y + 1) } else { (x, y + 1) }
        }
    } else if x == y {
        (x + 1, 0)
    } else {
        (x + 1, y)
    }
}
fn get_filename(words: &[usize; COLUMNS], positions: &Vec<(u8, u8)>) -> String {
    if NUM_QUEUE_LETTERS == 0 {
        return String::from(format!("{}x{}.txt", ROWS, COLUMNS));
    }
    let mut output = String::new();
    for i in 0..NUM_QUEUE_LETTERS {
        let (x, y) = positions[i];
        output.push(((words[x as usize] >> (5 * y) & 31) + 65) as u8 as char);
    }

    output.push_str(".txt");
    output
}

fn to_string(words: &[usize; COLUMNS]) -> String {
    (0..ROWS)
        .map(|y| {
            (0..COLUMNS)
                .map(|x| ((words[x] >> (5 * y) & 31) + 65) as u8 as char)
                .collect::<String>()
        })
        .join("\n")
}
