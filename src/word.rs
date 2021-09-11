use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::BufRead;

/// Words are represented by a lookup table. This is an index into that table. More common words are "lesser" by the Ord trait.
#[derive(Eq, PartialEq,Debug,Ord, PartialOrd,Copy, Clone)]
pub struct WordIndex(usize);


pub struct Words {
    words : Vec<String>,
    lookup : HashMap<String,WordIndex>,
}

impl Words {
    pub fn word(&self,index:WordIndex) -> &str { &self.words[index.0]}

    pub fn index(&self,word:&str) -> Option<WordIndex> { self.lookup.get(word).cloned() }

    fn add(&mut self,s:&str) -> WordIndex {
        let res = WordIndex(self.words.len());
        self.words.push(s.to_string());
        self.lookup.insert(s.to_string(),res);
        res
    }

    pub fn all_indices(&self) -> impl Iterator<Item=WordIndex> {
        (0..self.words.len()).map(|i|WordIndex(i))
    }

}

pub struct WordVec {
    vec : Vec<f64>,
    mag : f64
}

fn dot_product(v1:&[f64],v2:&[f64]) -> f64 {
    assert_eq!(v1.len(),v2.len());
    let mut res = 0.0;
    for i in 0..v1.len() {
        res+=v1[i]*v2[i];
    }
    res
}
fn distance(v1:&[f64],v2:&[f64]) -> f64 {
    assert_eq!(v1.len(),v2.len());
    let mut res = 0.0;
    for i in 0..v1.len() {
        let d = v1[i]-v2[i];
        res+=d*d;
    }
    res.sqrt()
}


impl WordVec {
    //fn len(&self) -> usize { self.vec.len() }

    pub fn dot_product(&self,v2:&WordVec) -> f64 {
        dot_product(&self.vec,&v2.vec)
    }

    pub fn cosine(&self,v2:&WordVec) -> f64 {
        self.dot_product(v2)/(self.mag*v2.mag)
    }

    pub fn distance(&self,v2:&WordVec) -> f64 {
        distance(&self.vec,&v2.vec)
    }

    pub fn new(vec:Vec<f64>) -> Self {
        let mag = dot_product(&vec,&vec).sqrt();
        WordVec{ vec, mag }
    }
}


/// same indices as Words.
pub struct WordVecs {
    vecs : Vec<WordVec>,
}

impl WordVecs {
    pub fn get(&self,word:WordIndex) -> &WordVec { &self.vecs[word.0] }
}


pub fn read_glove<P:AsRef<Path>>(path:P) -> std::io::Result<(Words,WordVecs)> {
    let file = File::open(path)?;
    let mut words = Words{ words: vec![], lookup: Default::default() };
    let mut wordvecs=WordVecs{ vecs: vec![] };
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.split(' ').collect::<Vec<_>>();
        let word = line[0];
        words.add(word);
        let nums : Vec<f64> = line[1..].iter().map(|l|l.parse().unwrap()).collect();
        wordvecs.vecs.push(WordVec::new(nums))
    }
    Ok((words,wordvecs))
}