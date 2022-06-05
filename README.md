# Main development moved

As of the prior commit 036713da, this has been moved into the right_to_ask_server git repository.

# Question Comparison

This is a Rust program as a prototype of the ability to compare questions to
find related questions.

It comprises two crates:
 * *word_comparison* which is the main library module,
 * *word_comparison_demo_webserver* which is a tiny webserver exposing the
   main API functions for a simple demo/testing.

## Algorithm used.

The algorithm used is a very simple keyword matching, with no attempt
to understand grammar or use word context.

First each question is parsed into separate tokens, which are generally
words or punctuation (see [SentencePart](word_comparison/src/sentences.rs)), 
which fall into one of the three following categories:
* [Listed keywords](word_comparison/src/listed_keywords.rs). These are special words or phrases that are likely to be
  politically important, and so are specially recognised. There may be multiple
  representations of these which are functionally synonymous, such as
  "Scott Morrison", "Prime Minister" or "ScoMo". These are anticipated to be
  hand maintained, and of modest size.
  
* [General Vocabulary](word_comparison/src/word_file.rs). These are general known words, with known synonyms,
  frequency of use and, for each synonym, a "goodness" score. This includes punctuation.
  This is assumed to be large (hundreds of thousands of words). Synonyms usually include different
  forms of the same word. No stemming is done.
  
* Unknown Words. These are unrecognised words. They may be typos, they may be vital domain
  specific vocabulary.
  
Tokens are generated by the following algorithm (see [TokenizedSentence::tokenize](word_comparison/src/sentences.rs)).
 * Convert everything to lower case.
 * While there is some question left
   * See if any listed keywords are a valid prefix. If so, extract that as a token.
   * Otherwise see if any general vocabulary is a valid prefix. If so, extract the longest one as a token.
   * Otherwise extract unknown word as characters until whitespace, not counting trailing punctuation (unless that is all there is).

Hashtags will generally be extracted as unknown words by this algorithm, which will be 
generally what is wanted.

Note that it may be worth checking to see if a listed keyword is also a general vocabulary word,
and if so including both in the tokenized result. This is currently not done but possibly should be.
    
Question *A*'s similarity to another question *B* is scored by adding up the score for each token in
*A*. A token with no match in *B* is given a score of 0. A listed keyword or unknown word with
a match in *B* is given a score of 10. A general vocabulary word in A with a perfect match in
*B* is given a score of 1 to 8 depending on how rare the word is 
(see [QuestionDatabase::score_known](word_comparison/src/comparison_list.rs)). Imperfect
matches via synonymns have this score reduced by the synonym goodness factor. 

Note that this similarity score is asymmetric - repeated tokens in the source get scored
multiple times, but not so repeated tokens in the reference question. This is because repeated
words in the source are presumably important to the person doing the query, whereas someone who
wrote a question about covid and vaccines would not want an existing repetitive question 
"I hate covid. I hate covid. I hate covid." to score higher than something mentioning both 
covid and vaccines.

### Implementation choices.

For efficiency, one doesn't want to scan every question for a score. Instead, each question is
tokenized when received, and is indexed by tokens. So one can efficiently get all questions matching
at least one token. The most common 400 general vocabulary words are not indexed. We don't want
to consider questions that only match by having a question mark, or the word "Why".

A set of candidate questions is produced by looking for all indexed matches to listed keywords,
general vocabulary, synonyms of general vocabulary, and unknown words. Each question in this
set is then scored and the resulting list is sorted. 

The small listed keywords are all stored in memory and exhaustively searched upon.

General vocabulary is much larger and exhaustive search is prohibitive. The obvious thing to
do would be to use some sort of hash table, but this is somewhat complicated by the desire to
get a prefix. To resolve this, an alphabetical list of all words is searched using a modified binary 
search algorithm. The modification is necessary as sometimes when you compare the reference question to
a word in the dictionary, you may find a common prefix in both, but divergent characters afterwords.
The best word may be earlier in the dictionary - the common prefix, or possibly later - a longer
prefix. See [WordsInFile::index_starting_between](word_comparison/src/word_file.rs) for details.

In order to provide fast startup and fast access, a memory mapped binary file is used for the general
vocabulary list. See [write_word_file](word_comparison/src/word_file.rs) for comments for
file format details. This has the drawback that if you modify this file while the program is running,
you unleash undefined behaviour (if you are lucky, a crash). Don't do this.

The list of questions in [the main api](word_comparison/src/comparison_list.rs) is not 
production ready, being there stored in memory backed by a text file, rather than a database.

# Running

This is a rust program. Make sure rust is installed on your computer. Version 1.54 or later is
recommended.

Compile from the directory containing this README.md with
```bash
cargo build --release
```

## Listed Keywords file

You need a file describing the listed keywords, called `ListedKeywords.csv`. This is
a csv file with one line per keyword, and commas separating different ways of 
referring to the same concept, e.g. 
```text
Covid,covid-19,covid 19,covid19,Coronavirus
Prime Minister,Scott Morrison,ScoMo,Scotty from Marketing
```

## General Vocabulary file

The general vocabulary file is in a file called `GeneralVocabulary.bin`
This is more complex as it is a large binary file. There is a utility to create this 
which will be built into the program `target/release/get_nearest_words`. This takes
a file that associates vectors with words [e.g. the pretrained word vectors at GloVe](https://nlp.stanford.edu/projects/glove/).
Download e.g. `glove.6B.zip`, unzip, and run a command like
```bash
./target/release/get_nearest_words create path_to_extracted_files/glove.6B.50d.txt
```
This uses the smallest file available. Presumably better results are obtained with larger
vocabularies or vector sizes, and are recommended. The listed 50d file is the fastest, not
the best. This will take some hours to run, and will create the file `GeneralVocabulary.bin`
in the current directory. Test it with
```bash
./target/release/get_nearest_words test
```
which will print out a list of synonyms. Stop it with control C when you have seen enough.

This program gets the 20 highest correlated (dot product, divided by magnitude, often referred to as cosine)
non-identical words as the synonyms.

The program does not currently do anything sensible with the cased downloads.

## Running the demo webserver

Run
```bash
./target/release/word_comparison_demo_webserver
```
in the directory containing the files `GeneralVocabulary.bin` and `ListedKeywords.csv`. 
Then open a web browser at [http://localhost:8091]. Type questions, and add them to the
list by pressing Enter or the Add button (which will clear the text box). New questions 
you type in will be compared to existing questions.

The server will store files in the text file "QuestionDatabase.txt" in the current directory.

Stop the server with control C.

## License

Copyright 2021 Thinking Cybersecurity Pty. Ltd.

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
