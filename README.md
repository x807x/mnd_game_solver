# MND Game Solver

[Game Link](https://game.mnd.gov.tw)

## Usage

1. Install [firefox](https://www.mozilla.org/zh-TW/firefox/new/) and [rust](https://www.rust-lang.org/tools/install) first

2. Install geckodriver
``` ps1
cargo install geckodriver
```

3. Install ocrs
``` ps1
cargo install ocrs
```

4. Try ocrs and it would automatically download recognition models
``` ps1
ocrs captcha.png
```

5. Run this program
``` ps1
cargo run --release -- --port 4445 -c 15
```