//! This file contains the actix web server wrapper around the functions



use actix_web::{HttpServer, middleware, web};
use actix_web::web::Json;
use actix_web::{get, post};
use async_std::sync::Mutex;
use word_comparison::word_file::{WordsInFile, WORD_MMAP_FILE};
use word_comparison::listed_keywords::ListedKeywords;
use word_comparison::comparison_list::{QuestionDatabase, QuestionID, ScoredIDs};
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct QueryQuestion {
    id : QuestionID,
}

/// Get some particular question
#[get("/get_question")]
async fn get_question(query:web::Query<QueryQuestion>, question_db: web::Data<Mutex<QuestionDatabase>>) -> Json<Option<String>> {
    Json(question_db.lock().await.lookup(query.id).map(|s|s.to_string()))
}

/// Get some particular question
#[get("/get_all_questions")]
async fn get_all_questions(question_db: web::Data<Mutex<QuestionDatabase>>) -> Json<Vec<String>> {
    Json(question_db.lock().await.get_all_questions())
}


#[derive(serde::Deserialize)]
struct QuerySimilarity {
    question : String,
}

/// Get some particular question
#[get("/get_similar")]
async fn get_similar(query:web::Query<QuerySimilarity>, question_db: web::Data<Mutex<QuestionDatabase>>, words: web::Data<WordsInFile>,keywords: web::Data<ListedKeywords>) -> Json<Vec<ScoredIDs>> {
    Json(question_db.lock().await.compare(&query.question,&words,&keywords))
}

/// find the path containing web resources, static web files that will be served.
/// This is usually in the directory `WebResources` but the program may be run from
/// other directories. To be as robust as possible it will try likely possibilities.
fn find_web_resources() -> PathBuf {
    let rel_here = std::path::Path::new(".").canonicalize().expect("Could not resolve path .");
    for p in rel_here.ancestors() {
        let pp = p.join("WebResources");
        if pp.is_dir() {return pp;}
        let pp = p.join("word_comparison_demo_webserver/WebResources");
        if pp.is_dir() {return pp;}
    }
    panic!("Could not find WebResources. Please run in a directory containing it.")
}


#[derive(serde::Deserialize)]
struct Publish {
    data : String,
}

#[post("/submit_question")]
async fn submit_question(command : web::Json<Publish>, question_db: web::Data<Mutex<QuestionDatabase>>, words: web::Data<WordsInFile>,keywords: web::Data<ListedKeywords>) -> Json<Result<QuestionID,String>> {
    Json(question_db.lock().await.add(&command.data,&words,&keywords).map_err(|e|e.to_string()))
}



#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    let words = WordsInFile::read_word_file(WORD_MMAP_FILE)?;
    let keywords = ListedKeywords::load(ListedKeywords::STD_LOCATION)?;
    let questions = QuestionDatabase::new(QuestionDatabase::STD_FILE_NAME,&words,&keywords)?;
    let questions = web::Data::new(Mutex::new(questions));
    let words = web::Data::new(words);
    let keywords = web::Data::new(keywords);
    println!("Running demo webserver on http://localhost:8091");
    HttpServer::new(move|| {
        actix_web::App::new()
            .app_data(questions.clone())
            .app_data(words.clone())
            .app_data(keywords.clone())
            .wrap(middleware::Compress::default())
            .service(get_question)
            .service(get_all_questions)
            .service(get_similar)
            .service(submit_question)
            .service(actix_files::Files::new("/", find_web_resources()).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8091")?
        .run()
        .await?;
    Ok(())
}