use fantoccini::{error::CmdError, Client, Locator};
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Question {
    pub stem: String,
    pub options: Vec<String>,
    pub answer: Option<u8>,
}

impl Question {
    pub async fn play(
        &mut self,
        client: &Client,
        database: &mut Vec<Self>,
    ) -> Result<(), CmdError> {
        if let Some(ans) = database.iter_mut().find(|d| d.stem == self.stem) {
            let answer = ans.options.get(ans.answer.unwrap() as usize).unwrap();
            let mut idx = 0;
            for option in &self.options {
                if option == answer {
                    self.answer = Some(idx);
                    break;
                }
                idx += 1;
            }
            if self.try_summit(client, idx).await? {
                return Ok(());
            }
        }
        for idx in 0..u8::try_from(self.options.len()).unwrap() {
            if self.try_summit(client, idx).await? {
                self.answer = Some(idx);
                break;
            }
        }
        database.push(self.clone());

        Ok(())
    }
    pub async fn try_summit(&mut self, client: &Client, guess_ans: u8) -> Result<bool, CmdError> {
        let ans_button = client
            .find(Locator::Id(format!("Ans{}", guess_ans + 1).as_str()))
            .await?;
        ans_button.click().await?;
        let Ok(keep_playing_button) = client.find(Locator::Id("imgBack2")).await else {
            return Ok(true);
        };
        if keep_playing_button.click().await.is_ok() {
            Ok(true)
        } else {
            let retry_button = client.find(Locator::Id("imgBack")).await?;
            retry_button.click().await?;
            Ok(false)
        }
    }

    pub async fn from(client: &Client) -> Result<Self, CmdError> {
        let stem = match client
            .find(Locator::Css(r#"p[class="display-5 question2"]"#))
            .await
        {
            Ok(stem) => stem,
            Err(err) => {
                warn!("Error: {}", err);
                return Err(err);
            }
        };

        let options = client
            .find_all(Locator::Css(
                r#"button[class="btn-answer btn-lg btn-block"]"#,
            ))
            .await?;
        let mut options_text: Vec<String> = Vec::new();
        for option in options {
            let text: String = option.text().await?.chars().skip(3).collect();
            options_text.push(text);
        }

        Ok(Self {
            stem: stem
                .text()
                .await?
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect(),
            options: options_text,
            answer: None,
        })
    }
}
