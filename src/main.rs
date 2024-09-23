use chrono::{Local, SecondsFormat};
use clap::Parser;
use colored::*;
use dotenv::dotenv;
use fantoccini::error::{CmdError, ErrorStatus};
use log::{info, warn, LevelFilter};
use ocrs::{OcrEngine, OcrEngineParams};
use player::Player;
use question::Question;
use rten::Model;
use std::{env, fs::File, io::Write, path::PathBuf, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::sleep,
};
mod player;
mod question;

const URL: &str = "https://game.mnd.gov.tw/gameindex.aspx";
const LOCALHOST: &str = "http://localhost";
const DATABASE: &str = "questions.json";

#[derive(Parser, Debug)]
struct Args {
    /// Port used for webdriver
    #[arg(long, short, default_value_t = 4444)]
    port: u16,

    #[arg(long, short)]
    cd_time: Option<f32>,
}

async fn run_geckodriver(args: &Args) {
    let mut child = Command::new("geckodriver.exe")
        .arg(format!("--port={}", args.port))
        .spawn()
        .expect("failed to execute process");

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                info!("geckodriver: {}", line);
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), CmdError> {
    let args = Args::parse();
    dotenv().ok();
    //    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    env_logger::Builder::new()
        .format(|buf, record| {
            let time = Local::now()
                .to_rfc3339_opts(SecondsFormat::Millis, true)
                .as_str()
                .bright_blue();
            let level = record.level().as_str();
            let colored_level = match record.level().to_level_filter() {
                LevelFilter::Info => level.green(),
                LevelFilter::Warn => level.yellow(),
                LevelFilter::Error => level.red(),
                _ => level.into(),
            };
            writeln!(buf, "{} [{}] - {}", time, colored_level, record.args(),)
        })
        .filter(None, LevelFilter::Info)
        .init();
    run_geckodriver(&args).await;

    info!("{}", "Start".green());

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
        let reader = std::io::BufReader::new(file);
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
                player.timer = match args.cd_time {
                    Some(cd_time) => Some(sleep(Duration::from_secs_f32(cd_time))),
                    None => None,
                };
                cnt += 1;
                info!("{}", format!("finish {} times", cnt).green());
            }
            Err(err) => match err {
                CmdError::WaitTimeout => {
                    continue;
                }
                CmdError::Standard(err) => match &err.error {
                    ErrorStatus::Timeout => {}
                    ErrorStatus::NoSuchElement => {}
                    ErrorStatus::InvalidSessionId => {
                        continue;
                    }
                    ErrorStatus::ElementNotInteractable => {
                        info!(
                            "Element {} Not Interactable",
                            err.message
                                .replace("Element ", "")
                                .replace(" could not be scrolled into view", "")
                        );
                    }
                    _ => {
                        warn!("Failed: {:?}", err);
                    }
                },
                _ => {
                    warn!("Failed: {:?}", err);
                }
            },
        }

        if database.len() > prev_database_len {
            let json = serde_json::to_string(&database).unwrap();
            let mut file = File::create(DATABASE)?;
            file.write_all(json.as_bytes())?;
            prev_database_len = database.len();
        }
    }
}
