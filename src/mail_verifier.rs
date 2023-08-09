use fantoccini::{ClientBuilder, Locator};

pub async fn get_mail_code(email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {

    let c = ClientBuilder::rustls().connect("http://localhost:9515").await?;
    let x: u32 = rand::random::<u32>() % 1920;
    let y = rand::random::<u32>() % 1080;
    c.set_window_position(x, y).await?;
    c.set_window_size(400, 400).await?;
    c.goto("https://mail.projectnoxius.com/webmail/").await?;
    c.find(Locator::Css("input[id='rcmloginuser']")).await?.send_keys(email).await?;
    c.find(Locator::Css("input[id='rcmloginpwd']")).await?.send_keys(password).await?;
    c.find(Locator::Css("button[id='rcmloginsubmit']")).await?.click().await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let verify_mail = c.find(Locator::LinkText("Your Rockstar Games verification code")).await?;
    let href = verify_mail.attr("href").await?.unwrap().to_string();
    c.goto(&href).await?;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let code = c.find(Locator::Css("p[class='v1rc-2fa-code v1rc-2fa-code-override']")).await?.text().await?;
    
    if code.len() != 6 {
        return Err("Code length is not 6".into());
    }

    c.close().await?;

    Ok(code.to_string())
}