use clap::Parser;
use dotenv::dotenv;
use env_logger::Env;
use fantoccini::error::CmdError;
use log::{info, warn};
use ocrs::{OcrEngine, OcrEngineParams};
use player::Player;
use question::Question;
use rten::Model;
use std::{
    env,
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};

mod player;
mod question;

const URL: &str = "https://game.mnd.gov.tw/gameindex.aspx";
const LOCALHOST: &str = "http://localhost";
const DATABASE: &str = "questions.json";

#[derive(Parser, Debug)]
struct Args {
    /// Port used for webdriver
    #[arg(long, default_value_t = 4444)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), CmdError> {
    let args = Args::parse();
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Start");

    let user_profile = env::var("USERPROFILE").unwrap();
    let mut detection_model_path = PathBuf::from(&user_profile);
    detection_model_path.push(".cache/ocrs/text-detection.rten");
    let detection_model = Model::load_file(&detection_model_path).unwrap();
    let mut recognition_model_path = PathBuf::from(&user_profile);
    recognition_model_path.push(".cache/ocrs/text-recognition.rten");
    let recognition_model = Model::load_file(&recognition_model_path).unwrap();
    let ocr_engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        //        alphabet: Some("0123456789".to_string()),
        ..Default::default()
    })
    .unwrap();

    let personal_id = env::var("personal_id").unwrap();
    let mut database: Vec<Question> = Vec::new();
    if let Ok(file) = File::open(DATABASE) {
        let reader = BufReader::new(file);
        database = serde_json::from_reader(reader)?;
    }
    let mut prev_database_len = database.len();

    let webdriver = format!("{}:{}", LOCALHOST, args.port);
    info!("Webdriver: {}", webdriver);
    let mut cnt: u128 = 0;
    let mut player = Player::new(URL, &webdriver).await?;

    loop {
        match player.play(&ocr_engine, &mut database, &personal_id).await {
            Ok(()) => {
                cnt += 1;
                info!("finish {} times", cnt);
            }
            Err(err) => {
                warn!("Failed: {}", err);
                if err.to_string() == "Tried to run command without establishing a connection" {
                    return Err(err);
                }
            }
        }

        if database.len() > prev_database_len {
            let json = serde_json::to_string(&database).unwrap();
            let mut file = File::create(DATABASE)?;
            file.write_all(json.as_bytes())?;
            prev_database_len = database.len();
        }
    }
}
