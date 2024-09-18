use dotenv::dotenv;
use env_logger::Env;
use fantoccini::error::CmdError;
use log::{info, warn};
use ocrs::{OcrEngine, OcrEngineParams};
use player::Player;
use question::Question;
use rten::Model;
use std::{env, path::PathBuf};

mod player;
mod question;

const URL: &str = "https://game.mnd.gov.tw";
const WEBDRIVER: &str = "http://localhost:4444";

#[tokio::main]
async fn main() -> Result<(), CmdError> {
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
    let mut ocr_engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        //        alphabet: Some("0123456789".to_string()),
        ..Default::default()
    })
    .unwrap();

    let personal_id = env::var("personal_id").unwrap();
    let mut database: Vec<Question> = Vec::new();
    let mut cnt: u128 = 0;
    let mut player = Player::new(URL, WEBDRIVER).await?;
    loop {
        match player
            .play(&mut ocr_engine, &mut database, &personal_id)
            .await
        {
            Ok(_) => {
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
    }
}
