use itertools::Itertools;
use std::fmt;
use std::fmt::Formatter;

pub struct TrieNode {
    pub has_children: bool,
    pub children: [Option<Box<TrieNode>>; 26],
}

impl TrieNode {
    pub fn from_words(sorted_words: &Vec<&str>) -> Self {
        let mut children = [const { None }; 26];
        if sorted_words.is_empty() || sorted_words[0].is_empty() {
            return TrieNode {
                has_children: false,
                children,
            };
        }

        let mut active_words = Vec::new();
        let mut active_letter = sorted_words[0].chars().next().unwrap() as u8 - 65;

        for word in sorted_words {
            if !word.starts_with((active_letter + 65) as char) {
                active_words.sort();
                children[active_letter as usize] = Some(Box::new(Self::from_words(&active_words)));
                active_letter = word.chars().next().unwrap() as u8 - 65;
                active_words.clear();
            }
            active_words.push(&word[1..]);
        }
        active_words.sort();
        children[active_letter as usize] = Some(Box::new(Self::from_words(&active_words)));

        TrieNode {
            has_children: true,
            children,
        }
    }

    pub fn leaf_nodes(&self) -> usize {
        if !self.has_children {
            return 1;
        }
        let mut total = 0;
        for i in 0..26 {
            if self.children[i].is_some() {
                total += self.children[i].as_ref().unwrap().leaf_nodes();
            }
        }

        total
    }
}

impl fmt::Debug for TrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let letters = (0..26usize)
            .filter(|&i| match self.children[i] {
                Some(_) => true,
                _ => false,
            })
            .map(|i| (i + 65) as u8 as char)
            .collect_vec();
        write!(f, "{letters:?}")
    }
}
