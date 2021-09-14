use std::collections::{HashMap, HashSet};
use crate::listed_keywords::{ListedKeywordIndex, ListedKeywords};
use crate::word::WordIndex;
use crate::word_file::WordsInFile;
use crate::sentences::{TokenizedSentence, SentencePart};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::io::Write;
use serde::{Serialize,Deserialize};
use std::collections::hash_map::Entry;

/// identifier for an old question.
#[derive(Copy, Clone,Eq, PartialEq,Debug,Hash,Serialize,Deserialize)]
pub struct QuestionID(usize);


struct StoredQuestion {
    question : String,
    keywords : HashSet<ListedKeywordIndex>,
    known_words : HashSet<WordIndex>,
    unique_words : HashSet<String>,
}
pub struct QuestionDatabase {
    filename : PathBuf,
    questions : Vec<StoredQuestion>,
    containing_keyword : HashMap<ListedKeywordIndex,Vec<QuestionID>>,
    containing_known_word : HashMap<WordIndex,Vec<QuestionID>>,
    containing_unique : HashMap<String,Vec<QuestionID>>,
}


impl StoredQuestion {
    pub fn new(question : String,words:&WordsInFile,keywords:&ListedKeywords) -> Self {
        let tokenized = TokenizedSentence::tokenize(&question,words,keywords);
        let mut keywords = HashSet::new();
        let mut known_words = HashSet::new();
        let mut unique_words = HashSet::new();
        for token in tokenized.parts {
            match token {
                SentencePart::Listed(word) => {keywords.insert(word);}
                SentencePart::Known(word) => { if word.0 > 400 { known_words.insert(word); }}
                SentencePart::Unknown(word) => {unique_words.insert(word);}
            }
        }
        StoredQuestion{ question, keywords, known_words, unique_words }
    }
}

impl QuestionDatabase {
    pub const STD_FILE_NAME : &'static str = "QuestionDatabase.txt";

    fn add_work(&mut self,question:String,words:&WordsInFile,keywords:&ListedKeywords) -> QuestionID {
        let question = StoredQuestion::new(question,words,keywords);
        let id = QuestionID(self.questions.len());
        fn add<K>(entry:Entry<K,Vec<QuestionID>>,id:QuestionID) {
            let v = entry.or_insert_with(||vec![]);
            if v.is_empty() || id!= *v.last().unwrap() { // make sure the same id is not included twice.
                v.push(id);
            }
        }
        for &word in &question.keywords {
            add(self.containing_keyword.entry(word),id);
        }
        for &word in &question.known_words {
            add(self.containing_known_word.entry(word),id);
            self.containing_known_word.entry(word).or_insert_with(||vec![]).push(id);
        }
        for word in &question.unique_words {
            add(self.containing_unique.entry(word.clone()),id);
            self.containing_unique.entry(word.clone()).or_insert_with(||vec![]).push(id);
        }
        self.questions.push(question);
        id
    }

    /// Add a new question to the database.
    pub fn add(&mut self,question:&str,words:&WordsInFile,keywords:&ListedKeywords) -> std::io::Result<QuestionID> {
        let question = question.replace('\n'," ");
        let mut file = OpenOptions::new().create(true).write(true).append(true).open(&self.filename)?;
        writeln!(file, "{}",question)?;
        let id = self.add_work(question,words,keywords);
        Ok(id)
    }

    /// Get a new database, initialised from text file if it exists.
    pub fn new<P:AsRef<Path>+ std::convert::AsRef<std::ffi::OsStr>>(path:P,words:&WordsInFile,keywords:&ListedKeywords) -> std::io::Result<Self> {
        let mut res = QuestionDatabase{
            filename : PathBuf::from(&path),
            questions: vec![],
            containing_keyword: Default::default(),
            containing_known_word: Default::default(),
            containing_unique: Default::default()
        };
        if let Ok(file) = File::open(path) {
            for line in std::io::BufReader::new(file).lines() {
                res.add_work(line?,words,keywords);
            }
        }
        Ok(res)
    }

    /// Get all questions in the database. Could be slow!
    pub fn get_all_questions(&self) -> Vec<String> {
        self.questions.iter().map(|q|q.question.clone()).collect()
    }
    /// Get the text associated with a question
    pub fn lookup(&self,id:QuestionID) -> Option<&str> {
        self.questions.get(id.0).map(|q|q.question.as_str())
    }

    const SCORE_KEYWORD : f64 = 10.0;
    const SCORE_UNIQUE : f64 = 10.0;
    fn score_known(word : WordIndex) -> f64 {
        if word.0 < 100 { 1.0 }
        else if word.0 < 500 { 2.0 }
        else if word.0 < 1000 { 3.0 }
        else if word.0 < 10000 { 4.0 }
        else if word.0 < 100000 { 6.0 }
        else { 8.0 }
    }

    pub fn compare(&self, question:&str, words:&WordsInFile, keywords:&ListedKeywords) -> Vec<ScoredIDs> {
        let tokenized = TokenizedSentence::tokenize(&question,words,keywords);
        println!();
        tokenized.explain(words,keywords);
        let mut scores = SentenceScores::default();
        for token in &tokenized.parts {
            match token {
                SentencePart::Listed(word) => {
                    scores.add_maybe(self.containing_keyword.get(word),Self::SCORE_KEYWORD);
                },
                SentencePart::Known(word) => {
                    if word.0>100 {
                        let score = Self::score_known(*word);
                        let mut avoid_twice = HashSet::new();
                        scores.add_maybe_avoid_counting_twice(self.containing_known_word.get(word),score,&mut avoid_twice);
                        for e in words.synonyms(*word) {
                            scores.add_maybe_avoid_counting_twice(self.containing_known_word.get(&e.word),score*e.value as f64,&mut avoid_twice);
                        }
                    }
                }
                SentencePart::Unknown(word) => {
                    scores.add_maybe(self.containing_unique.get(word),Self::SCORE_UNIQUE);
                },
            }
        }
        scores.extract_ordered()
    }

}

#[derive(Default)]
struct SentenceScores {
    scores : HashMap<QuestionID,f64>,
}

#[derive(Copy, Clone,Debug,Serialize,Deserialize)]
pub struct ScoredIDs {
    pub id : QuestionID,
    pub score : f64,
}
impl SentenceScores {
    /// Add a set of questions containing this id.
    /// Assign the given number of points.
    fn add_maybe(&mut self,qs:Option<&Vec<QuestionID>>,points:f64) {
        if let Some(qs) = qs {
            for &q in qs {
                *self.scores.entry(q).or_insert(0.0)+=points;
            }
        }
    }

    /// like add_maybe, but
    /// Don't assign points if it is in the avoid_twice optional list (and add ones that you do add points for).
    fn add_maybe_avoid_counting_twice(&mut self,qs:Option<&Vec<QuestionID>>,points:f64,avoid_twice:&mut HashSet<QuestionID>) {
        if let Some(qs) = qs {
            for &q in qs {
                if avoid_twice.insert(q) {
                    *self.scores.entry(q).or_insert(0.0)+=points;
                }
            }
        }
    }

    pub fn extract_ordered(self) -> Vec<ScoredIDs> {
        let mut res : Vec<ScoredIDs> = self.scores.iter().map(|(&id,&score)|ScoredIDs{id,score}).collect();
        res.sort_by(|a,b|b.score.partial_cmp(&a.score).unwrap());
        res
    }
}