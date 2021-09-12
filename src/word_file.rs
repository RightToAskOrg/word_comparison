//! Store words in a file in a way that can be looked up quickly with mmap.



use crate::word::{MemoryWords, WordVecs, WordIndex, WordSource};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use crate::near_words::{SmallestN, WordAndValue};
use memmap::Mmap;
use std::cmp::Ordering;

pub const WORD_MMAP_FILE : &str = "/big/shared/NLP.glove/glove6B/glove.6B.100d.synonyms";


/// * File format
/// All things are little endian.
/// Word identifiers are 4 bytes.
///
/// 4 Bytes : Ascii "WORD"
/// 4 bytes : Number of words (N).
/// 4 bytes : Number of synonyms (n).
/// N * (n * (4+4bytes)) : for each word i, the n best synonyms, in order best to worst, each one being a word identifer (4 bytes) and a correlation score (f32 4 bytes)
/// N * 4bytes : Word identifier i, in alphabetical order.
/// N * 4bytes : Offset of word i, relative to after this section.
/// 4 bytes : Length of this section (N strings) in bytes
/// base if where offset of word i is relative to
/// N * utf-8 nul terminated strings, being words referred to above.
pub fn write_word_file<P:AsRef<Path>>(path:P,words:&MemoryWords,wordvecs:&WordVecs,num_synonyms:u32) -> std::io::Result<()>{
    let mut file = File::create(path)?;
    file.write_all("WORD".as_bytes())?;
    file.write_all(&(words.len() as u32).to_le_bytes())?;
    file.write_all(&num_synonyms.to_le_bytes())?;
    for word_index in words.all_indices() {
        let vec_i = wordvecs.get(word_index);
        let mut best_cosine = SmallestN::new(num_synonyms as usize);
        for comparison in words.all_indices() {
            if comparison!=word_index {
                best_cosine.add(WordAndValue{ word: comparison, value: -vec_i.cosine(wordvecs.get(comparison)) as f32 });
            }
        }
        let best_cosine = best_cosine.into_sorted_vec();
        for synonym in best_cosine {
            file.write_all(&(synonym.word.0 as u32).to_le_bytes())?;
            file.write_all(&(-synonym.value as f32).to_le_bytes())?;
        }
    }
    let mut alphabetical_order : Vec<WordIndex> = words.all_indices().collect();
    alphabetical_order.sort_by_key(|w|words.word(*w));
    for w in alphabetical_order {
        file.write_all(&(w.0 as u32).to_le_bytes())?;
    }
    let mut word_text: Vec<u8> = vec![];
    for word_index in words.all_indices() {
        file.write_all(&(word_text.len() as u32).to_le_bytes())?;
        word_text.write_all(words.word(word_index).as_bytes())?;
        word_text.write_all(&[0u8])?;
    }
    file.write_all(&(word_text.len() as u32).to_le_bytes())?;
    file.write_all(&word_text)?;
    Ok(())
}


pub struct WordsInFile {
    mmap : Mmap,
    number_words: usize,
    num_synonyms : usize,
    synonyms_start : usize,
    alphabetic_order_start : usize,
    offsets_start : usize,
    strings_start : usize,
}

impl WordsInFile {
    /// Read the word file in a mmap mode - modifying the file while running will cause a crash!
    pub fn read_word_file<P:AsRef<Path>>(path:P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)?  };
        let four_array = |offset:usize| [mmap[offset],mmap[offset+1],mmap[offset+2],mmap[offset+3]];
        let read_u32 = |offset:usize| u32::from_le_bytes(four_array(offset));
        let num_words = read_u32(4) as usize;
        let num_synonyms = read_u32(8) as usize;
        let synonyms_start = 12;
        let alphabetic_order_start = synonyms_start+num_words*num_synonyms*8;
        let offsets_start = alphabetic_order_start+num_words*4;
        let strings_start = offsets_start+num_words*4+4;
        Ok(WordsInFile{
            mmap,
            number_words: num_words,
            num_synonyms,
            synonyms_start,
            alphabetic_order_start,
            offsets_start,
            strings_start
        })
    }
    fn four_array(&self,offset:usize) -> [u8;4] { [self.mmap[offset],self.mmap[offset+1],self.mmap[offset+2],self.mmap[offset+3]] }
    fn read_u32(&self,offset:usize) -> u32 { u32::from_le_bytes(self.four_array(offset)) }
    fn read_f32(&self,offset:usize) -> f32 { f32::from_le_bytes(self.four_array(offset)) }

    pub fn synonyms(&self,word:WordIndex) -> Vec<WordAndValue> {
        let mut offset = self.synonyms_start+word.0 as usize*(8*self.num_synonyms);
        let mut res = vec![];
        for _ in 0..self.num_synonyms {
            let word = WordIndex(self.read_u32(offset));
            offset+=4;
            let value = self.read_f32(offset);
            offset+=4;
            res.push(WordAndValue{ word, value });
        }
        res
    }

}

impl WordSource for WordsInFile {
    fn len(&self) -> usize { self.number_words }
    fn word(&self,index:WordIndex) -> &str {
        let start = self.strings_start+self.read_u32(self.offsets_start+4*index.0 as usize) as usize;
        let buf = &self.mmap[start..];
        let len = buf.iter().position(|b|*b==0).expect("String not null terminated");
        let buf = &buf[..len];
        std::str::from_utf8(buf).expect("String not utf-8")
    }

    fn index(&self,word:&str) -> Option<WordIndex> {
        let mut low  = 0; // values less than this are NOT the word.
        let mut high = self.number_words; // values equal to or higher than this are NOT the word.
        while low<=high {
            let mid = (low+high)/2;
            let word_index = WordIndex(self.read_u32(self.alphabetic_order_start+4*mid));
            let mid_word = self.word(word_index);
            match word.cmp(mid_word) {
                Ordering::Less => { high = mid }
                Ordering::Equal => { return Some(word_index) }
                Ordering::Greater => { low = mid+1 }
            }
        }
        None
    }

}