use std::time::Duration;

use fantoccini::{error::CmdError, Client, ClientBuilder, Locator};
use log::{info, warn};
use ocrs::{ImageSource, OcrEngine};
use tokio::time::sleep;

use crate::question::Question;
const CAPTCHA_PATH: &str = "captcha.png";

#[derive(Clone, Debug)]
pub struct Player {
    pub url: String,
    client: Client,
    current_question: Question,
}

impl Player {
    pub async fn new(url: &str, webdriver: &str) -> Result<Self, CmdError> {
        let player = Player {
            url: url.to_string(),
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
        ocr_engine: &mut OcrEngine,
        database: &mut Vec<Question>,
        personal_id: &str,
    ) -> Result<(), CmdError> {
        self.client.goto(&self.url).await?;
        let start_playing_btn = self
            .client
            .find(fantoccini::Locator::Css(
                r#"img[class="img-fluid mt-sm-3"]"#,
            ))
            .await?;
        start_playing_btn.click().await?;
        let start_playing_btn = self
            .client
            .find(fantoccini::Locator::Css(
                r#"img[class="img-fluid mt-sm-3"]"#,
            ))
            .await?;
        start_playing_btn.click().await?;
        for _ in 0..9 {
            self.current_question = Question::from(&self.client).await?;
            self.current_question.play(&self.client, database).await?;
        }
        self.input_data(ocr_engine, personal_id).await?;
        Ok(())
    }
    async fn input_data(
        &mut self,
        ocr_engine: &mut OcrEngine,
        personal_id: &str,
    ) -> Result<(), CmdError> {
        let personal_id_block = self.client.find(fantoccini::Locator::Id("PID")).await?;
        personal_id_block.send_keys(personal_id).await?;

        self.input_captcha(ocr_engine).await?;

        sleep(Duration::from_secs_f32(0.9)).await;
        let go_back_btn = self
            .client
            .find(fantoccini::Locator::Css(r#"img[alt="回首頁"]"#))
            .await?;
        go_back_btn.click().await?;
        sleep(Duration::from_secs_f32(0.5)).await;

        Ok(())
    }

    async fn input_captcha(&mut self, ocr_engine: &mut OcrEngine) -> Result<(), CmdError> {
        loop {
            self.solve_captcha(ocr_engine).await?;
            sleep(Duration::from_secs_f32(0.5)).await;

            self.confirm_captcha().await?;
            sleep(Duration::from_secs_f32(0.5)).await;
            match self.client.get_alert_text().await {
                Ok(alert) => {
                    info!("Alert!!! {}", alert);
                    if alert == "請稍後再試!" || alert == "所填寫的驗證碼與所給的不符"
                    {
                        self.client.accept_alert().await?;
                    }
                }
                Err(_) => {
                    return Ok(());
                }
            }
        }
    }

    async fn download_captcha_img(&mut self) -> Result<(), CmdError> {
        let captcha_img = self
            .client
            .find(fantoccini::Locator::Id("CAPTCHAImage"))
            .await?;
        let img = captcha_img.screenshot().await?;

        std::fs::write(CAPTCHA_PATH, img.clone()).unwrap();
        Ok(())
    }
    async fn captcha_ocr(
        &mut self,
        ocr_engine: &mut OcrEngine,
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
        let captcha_ans: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        if captcha_ans.len() == 6 {
            Ok(Some(captcha_ans))
        } else {
            info!("Bad CAPTCHA: {}", text);
            Ok(None)
        }
    }
    async fn reload_captcha(&mut self) -> Result<(), CmdError> {
        let reflash_btn = self.client.find(Locator::Id("reflash")).await?;
        reflash_btn.click().await
    }
    async fn solve_captcha(&mut self, ocr_engine: &mut OcrEngine) -> Result<(), CmdError> {
        loop {
            self.download_captcha_img().await?;
            match self.captcha_ocr(ocr_engine).await? {
                Some(ans) => {
                    info!("Captcha ans: {}", ans);
                    let captcha_input = self
                        .client
                        .find(fantoccini::Locator::Id("txtValidateCode"))
                        .await?;
                    captcha_input.clear().await?;
                    return captcha_input.send_keys(&ans).await;
                }
                _ => {
                    self.reload_captcha().await?;
                }
            }
        }
    }
    async fn confirm_captcha(&mut self) -> Result<(), CmdError> {
        let confirm_btn = self.client.find(fantoccini::Locator::Id("ImgBtn")).await?;
        confirm_btn.click().await
    }
}
