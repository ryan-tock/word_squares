use std::fmt;
use std::fmt::Formatter;
use itertools::Itertools;

pub struct TrieNode {
    pub children: Box<[Option<TrieNode>; 26]>,
}
impl TrieNode {
    pub fn new(sorted_words: &Vec<&str>) -> Self {
        if sorted_words[0].len() == 0 {
            return TrieNode {
                children: Box::new(std::array::from_fn(|_| None)),
            }
        }
        let mut children: Box<[Option<TrieNode>; 26]> = Box::new(std::array::from_fn(|_| None));
        let mut active_letter = sorted_words[0].chars().next().unwrap();
        let mut active_words = Vec::new();

        for &word in sorted_words {
            if word.chars().next().unwrap() == active_letter {
                active_words.push(&word[1..]);
            } else {
                active_words.sort();
                let new_node = TrieNode::new(&active_words);
                children[active_letter as usize - 65] = Some(new_node);
                active_letter = word.chars().next().unwrap();
                active_words = vec!(&word[1..]);
            }
        }
        active_words.sort();
        let new_node = TrieNode::new(&active_words);
        children[active_letter as usize - 65] = Some(new_node);

        TrieNode {
            children,
        }
    }
}

impl fmt::Debug for TrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let letters = (0..26).filter(|i| match self.children[*i] {Some(_) => true, _ => false}).map(|i| (i + 65) as u8 as char).collect_vec();
        write!(f, "{letters:?}")
    }
}