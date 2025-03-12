use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::{fs, io, thread};
use std::time::Instant;
use itertools::Itertools;
use crossbeam_channel::{bounded, Sender};
use crate::trie::TrieNode;

mod trie;
const WORDS: &str = include_str!("words.txt");

const SIZE_X: usize = 4;
const SIZE_Y: usize = 5;
const PRINT_SOLUTIONS: bool = true;
const THREADS: usize = 4;
const NUM_QUEUE_LETTERS: usize = 2;

fn main() {
    let start = Instant::now();

    let words_x = WORDS.lines().filter(|w| w.len() == SIZE_X).collect_vec();
    let words_y = WORDS.lines().filter(|w| w.len() == SIZE_Y).collect_vec();
    let (words_x, words_y) = if SIZE_X == SIZE_Y {
        drop(words_y);
        (&words_x, &words_x)
    } else {
        (&words_x, &words_y)
    };
    if words_x.len() == 0 || words_y.len() == 0 {
        println!("Found 0 {SIZE_X}x{SIZE_Y} word grids in {:?}", start.elapsed());
        return;
    }

    let trie_x = TrieNode::from_words(words_x);
    let trie_y = TrieNode::from_words(words_y);
    let (trie_x, trie_y) = if SIZE_X == SIZE_Y {
        drop(trie_y);
        (&trie_x, &trie_x)
    } else {
        (&trie_x, &trie_y)
    };

    let queue = generate_queue(trie_x, std::array::from_fn(|_| trie_y), 0, NUM_QUEUE_LETTERS);
    let prefixes_ordered = queue.iter().map(|(x, _, _)| x.clone()).collect_vec();
    let queue_sorted = queue.into_iter().sorted_by(|(_, node_1, _), (_, node_2, _)| node_1.children.iter().filter(|x| x.is_some()).count().cmp(&node_2.children.iter().filter(|x| x.is_some()).count()).reverse()).collect_vec();
    let work_queue = Arc::new(Mutex::new(queue_sorted.iter()));
    let solutions = Arc::new(Mutex::new(0usize));

    thread::scope(|scope| {
        for _ in 0..THREADS {
            let (s, r) = bounded::<[usize; SIZE_Y]>(1024);
            let solutions = solutions.clone();
            let work_queue = work_queue.clone();
            scope.spawn(move || {
                loop {
                    let work_queue = Arc::clone(&work_queue);
                    let task = work_queue.lock().unwrap().next();

                    match task {
                        Some(_) => {},
                        None => break,
                    }

                    let (word, node, nodes_y) = task.unwrap();
                    drop(work_queue);

                    s.send(std::array::from_fn(|i| if i == 0 {usize::MAX} else {*word})).unwrap();

                    let mut start_nodes_x = std::array::from_fn(|_| trie_x);
                    start_nodes_x[0] = node;

                    let mut start_nodes_y = std::array::from_fn(|_| trie_y);
                    for i in 0..NUM_QUEUE_LETTERS {
                        start_nodes_y[i] = nodes_y[i];
                    }

                    let mut start_words = [0usize; SIZE_Y];
                    start_words[0] = *word;

                    let found_solutions = generate_grids(start_nodes_x, start_nodes_y, NUM_QUEUE_LETTERS as u8, 0, start_words, &s);

                    let solutions = Arc::clone(&solutions);
                    *solutions.lock().unwrap() += found_solutions;
                    drop(solutions);
                }
            });
            scope.spawn(move || {
                let mut output_file = File::open("src/main.rs").unwrap();
                let mut writer = BufWriter::new(output_file);
                for words in r.iter() {
                    if words[0] == usize::MAX && PRINT_SOLUTIONS {
                        let mut output_name = String::new();
                        output_name.push_str(to_string_sized(words[1], NUM_QUEUE_LETTERS).as_str());
                        output_name.push_str(".txt");
                        output_file = File::create(output_name).unwrap();
                        writer = BufWriter::new(output_file);
                    } else {
                        writer.write(words.map(|word| to_string(word)).join("\n").as_bytes()).unwrap();
                        writer.write("\n\n".as_bytes()).unwrap();
                    }
                }
            });
        }
    });
    let solutions = *solutions.lock().unwrap();

    if PRINT_SOLUTIONS {
        let mut output_name = String::new();
        output_name.push_str(SIZE_X.to_string().as_str());
        output_name.push_str("x");
        output_name.push_str(SIZE_Y.to_string().as_str());
        output_name.push_str(".txt");
        let mut output_file = File::create(&output_name).unwrap();

        for &word in prefixes_ordered.iter() {
            let mut output_name = String::new();
            output_name.push_str(to_string_sized(word, NUM_QUEUE_LETTERS).as_str());
            output_name.push_str(".txt");
            let mut file = File::open(&output_name).unwrap();
            io::copy(&mut file, &mut output_file).unwrap();
            fs::remove_file(output_name).unwrap();
        }

        if solutions == 0 {
            fs::remove_file(&output_name).unwrap();
        }
    }

    println!("Found {} {SIZE_X}x{SIZE_Y} word grids in {:?}", solutions, start.elapsed());
}

fn generate_grids(nodes_x: [&TrieNode; SIZE_Y], nodes_y: [&TrieNode; SIZE_X], x: u8, y: u8, words: [usize; SIZE_Y], solutions: &Sender<[usize; SIZE_Y]>) -> usize {
    let y = if x as usize >= SIZE_X {y+1} else {y};
    let x = if x as usize >= SIZE_X {0} else {x};
    if y as usize >= SIZE_Y {
        if PRINT_SOLUTIONS {
            solutions.send(words).unwrap();
        }
        return 1;
    }

    let mut total = 0;
    for i in 0..26 {
        match nodes_x[y as usize].children[i] {
            Some(_) => {},
            None => {continue;},
        }
        match nodes_y[x as usize].children[i] {
            Some(_) => {},
            None => {continue;},
        }

        let mut new_words = words;
        new_words[y as usize] = (new_words[y as usize] << 5) + i;

        let mut new_x = nodes_x;
        new_x[y as usize] = new_x[y as usize].children[i].as_ref().unwrap();

        let mut new_y = nodes_y;
        new_y[x as usize] = new_y[x as usize].children[i].as_ref().unwrap();

        total += generate_grids(new_x, new_y, x+1, y, new_words, solutions);
    }

    total
}

fn generate_queue<'a>(node_x: &'a TrieNode, nodes_y: [&'a TrieNode; NUM_QUEUE_LETTERS], word: usize, queue_letters: usize) -> Vec<(usize, &'a TrieNode, [&'a TrieNode; NUM_QUEUE_LETTERS])> {
    if queue_letters == 0 {
        return vec![(word, node_x, nodes_y)];
    }
    let mut solutions = Vec::new();
    let depth = NUM_QUEUE_LETTERS - queue_letters;
    for letter in 0..26 {
        match nodes_y[depth].children[letter].as_ref() {
            Some(_) => {},
            None => {continue},
        }
        match node_x.children[letter].as_ref() {
            Some(child) => {
                let mut new_nodes_y = nodes_y.clone();
                new_nodes_y[depth] = new_nodes_y[depth].children[letter].as_ref().unwrap();
                solutions.extend(generate_queue(child, new_nodes_y, (word << 5) + letter, queue_letters - 1))
            },
            None => {},
        }
    }

    solutions
}

fn to_string_sized(word: usize, size: usize) -> String {
    (0..size).map(|i| (((word >> (i * 5)) & 31) + 65) as u8 as char).rev().collect::<String>()
}

fn to_string(word: usize) -> String {
    to_string_sized(word, SIZE_X)
}
