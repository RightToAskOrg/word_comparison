//! Get words closest to other words.


use word_comparison::word::{read_glove, WordIndex, Words};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, PartialEq,Copy, Clone)]
struct WordAndValue {
    word : WordIndex,
    value : f64,
}

impl Eq for WordAndValue {
}
impl Ord for WordAndValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.value.partial_cmp(&other.value).unwrap() {
            Ordering::Equal => self.word.cmp(&other.word),
            res => res,
        }
    }
}
impl PartialOrd for WordAndValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

/// Stores the smallest n values
struct SmallestN {
    n : usize,
    values : BinaryHeap<WordAndValue>,
}

impl SmallestN {
    pub fn new(n:usize) -> Self {
        SmallestN{n,values:BinaryHeap::with_capacity(n+1)}
    }

    pub fn add(&mut self,w:WordAndValue) {
        if self.values.len()<self.n || (w.value<self.values.peek().unwrap().value && {self.values.pop(); true}) {
            self.values.push(w);
        }
    }

    pub fn into_sorted_vec(self) -> Vec<WordAndValue> { self.values.into_sorted_vec() }
}

fn print_vec(words:&Words,v:&[WordAndValue],reversed_sign:bool) -> String {
    let mut res = String::new();
    for e in v {
        res.push('\t');
        res.push_str(words.word(e.word));
        res.push('\t');
        let value = if reversed_sign { -e.value } else { e.value };
        res.push_str(&format!("{:.4}",value))
    }
    res
}

fn main() -> std::io::Result<()>{
    let (words,wordvecs) = read_glove("/big/shared/NLP.glove/glove6B/glove.6B.50d.txt")?;
    for i in words.all_indices() {
        let word = words.word(i);
        let vec_i = wordvecs.get(i);
        let mut best_dot = SmallestN::new(10);
        let mut best_cosine = SmallestN::new(10);
        let mut best_distance = SmallestN::new(10);
        for j in words.all_indices() {
            let vec_j = wordvecs.get(j);
            best_dot.add(WordAndValue{ word: j, value: -vec_i.dot_product(vec_j) });
            best_cosine.add(WordAndValue{ word: j, value: -vec_i.cosine(vec_j) });
            best_distance.add(WordAndValue{ word: j, value: vec_i.distance(vec_j) })
        }
        let best_dot = best_dot.into_sorted_vec();
        let best_cosine = best_cosine.into_sorted_vec();
        let best_distance = best_distance.into_sorted_vec();
        println!("Word {}",word);
        println!("Dot{}",print_vec(&words,&best_dot,true));
        println!("Cos{}",print_vec(&words,&best_cosine,true));
        println!("D{}",print_vec(&words,&best_distance,false));
        println!();
    }

    Ok(())
}