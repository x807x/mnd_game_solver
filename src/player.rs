use std::time::Duration;

use colored::Colorize;
use fantoccini::{error::CmdError, Client, ClientBuilder, Locator};
use log::{info, warn};
use ocrs::{ImageSource, OcrEngine};
use tokio::time::{sleep, Sleep};

use crate::question::Question;
const CAPTCHA_PATH: &str = "captcha.png";
const CD_TIME: f32 = 60.0;
const DOWNLOAD_WAIT: f32 = 0.0;
const RELOAD_WAIT: f32 = 0.15;

#[derive(Clone, Debug)]
pub struct Player {
    pub url: String,
    client: Client,
    current_question: Question,
}

impl Player {
    pub async fn new(url: &str, webdriver: &str) -> Result<Self, CmdError> {
        let player = Self {
            url: url.to_owned(),
            client: ClientBuilder::native()
                .connect(webdriver)
                .await
                .expect("You have install firefox"),
            current_question: Question::default(),
        };
        Ok(player)
    }

    pub async fn play(
        &mut self,
        ocr_engine: &OcrEngine,
        database: &mut Vec<Question>,
        personal_id: &str,
    ) -> Result<(), CmdError> {
        let timer = sleep(Duration::from_secs_f32(CD_TIME));
        self.client.goto(&self.url).await?;
        let start_playing_btn = self
            .client
            .find(Locator::Css(r#"img[class="img-fluid mt-sm-3"]"#))
            .await?;
        start_playing_btn.click().await?;
        for _ in 0..9 {
            self.current_question = Question::from(&self.client).await?;
            self.current_question.play(&self.client, database).await?;
        }
        self.input_data(ocr_engine, personal_id, timer).await?;
        Ok(())
    }

    async fn input_data(
        &mut self,
        ocr_engine: &OcrEngine,
        personal_id: &str,
        timer: Sleep,
    ) -> Result<(), CmdError> {
        let personal_id_block = self.client.find(Locator::Id("PID")).await?;
        personal_id_block.send_keys(personal_id).await?;

        self.input_captcha(ocr_engine, true).await?;
        timer.await;
        while !self.confirm_captcha().await? {
            self.reload_captcha().await?;
            self.input_captcha(ocr_engine, false).await?;
        }

        Ok(())
    }

    async fn input_captcha(
        &mut self,
        ocr_engine: &OcrEngine,
        strict: bool,
    ) -> Result<(), CmdError> {
        info!("Start solving CAPTCHA");
        self.solve_captcha(ocr_engine, strict).await
    }

    async fn download_captcha_img(&mut self) -> Result<(), CmdError> {
        let captcha_img = self.client.find(Locator::Id("CAPTCHAImage")).await?;
        let img = captcha_img.screenshot().await?;

        std::fs::write(CAPTCHA_PATH, img.clone()).unwrap();
        sleep(Duration::from_secs_f32(DOWNLOAD_WAIT)).await;
        Ok(())
    }

    async fn captcha_ocr(
        &mut self,
        ocr_engine: &OcrEngine,
        strict: bool,
    ) -> Result<Option<String>, CmdError> {
        let image = match image::open(CAPTCHA_PATH) {
            Ok(image) => image.to_rgb8(),
            Err(err) => {
                warn!("Image err: {}", err);
                self.download_captcha_img().await?;
                return Ok(None);
            }
        };
        let img_source = ImageSource::from_bytes(image.as_raw(), image.dimensions()).unwrap();
        let ocr_input = ocr_engine.prepare_input(img_source).unwrap();
        let text = ocr_engine.get_text(&ocr_input).unwrap();
        let captcha_ans: String = text.chars().filter(char::is_ascii_digit).collect();
        if captcha_ans.len() == 6 && (text.len() == 6 || !strict) {
            Ok(Some(captcha_ans))
        } else {
            info!("Bad CAPTCHA: {:?}", text);
            Ok(None)
        }
    }

    async fn reload_captcha(&mut self) -> Result<(), CmdError> {
        let reflash_btn = self.client.find(Locator::Id("reflash")).await?;
        reflash_btn.click().await?;
        sleep(Duration::from_secs_f32(RELOAD_WAIT)).await;
        Ok(())
    }

    async fn solve_captcha(
        &mut self,
        ocr_engine: &OcrEngine,
        strict: bool,
    ) -> Result<(), CmdError> {
        loop {
            self.download_captcha_img().await?;
            if let Some(ans) = self.captcha_ocr(ocr_engine, strict).await? {
                info!("Captcha ans: {}", ans);
                let captcha_input = self.client.find(Locator::Id("txtValidateCode")).await?;
                captcha_input.clear().await?;
                return captcha_input.send_keys(&ans).await;
            }
            self.reload_captcha().await?;
        }
    }

    async fn confirm_captcha(&mut self) -> Result<bool, CmdError> {
        let confirm_btn = self.client.find(Locator::Id("ImgBtn")).await?;
        confirm_btn.click().await?;
        match self.client.get_alert_text().await {
            Ok(alert) => {
                warn!("{} {}", "Alert!!!".red(), alert);
                if alert == "請稍後再試!" || alert == "所填寫的驗證碼與所給的不符"
                {
                    self.client.accept_alert().await?;
                }
                Ok(false)
            }
            Err(_) => Ok(true),
        }
    }
}
