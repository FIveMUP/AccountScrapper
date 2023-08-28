use fantoccini::{ClientBuilder, Locator};

pub async fn get_mail_code(email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
    let c = ClientBuilder::rustls().connect("http://localhost:9515").await?;
    let x: u32 = (rand::random::<u32>() % 1000) + 100;
    let y = (rand::random::<u32>() % 850) + 100;
    c.set_window_position(x, y).await?;
    c.set_window_size(500, 500).await?;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    c.goto("http://mail2web.com/webmail/login.aspx").await?;
    c.find(Locator::Css("input[name='emailaddress']")).await?.send_keys(email).await?;
    c.find(Locator::Css("input[name='password']")).await?.send_keys(password).await?;
    c.find(Locator::Css("input[name='btnG']")).await?.click().await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Logged in");
    let verify_mail = c.find(Locator::LinkText("Your Rockstar Games verification code")).await?;
    let href = verify_mail.attr("href").await?.unwrap().to_string();
    c.goto(&href).await?;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let html_dump = c.source().await?;

    // regex of <br>236128 to extract number
    let re = regex::Regex::new(r"<br>(\d{6})").unwrap();
    let code = re.captures(&html_dump).unwrap()[1].to_string();

    c.close().await?;

    Ok(code.to_string())
}

pub async fn verify_mail(email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {

    let c = ClientBuilder::rustls().connect("http://localhost:9515").await?;
    let x: u32 = (rand::random::<u32>() % 1000) + 100;
    let y = (rand::random::<u32>() % 850) + 100;
    c.set_window_position(x, y).await?;
    c.set_window_size(500, 500).await?;
    c.goto("http://mail2web.com/webmail/login.aspx").await?;
    c.find(Locator::Css("input[name='emailaddress']")).await?.send_keys(email).await?;
    c.find(Locator::Css("input[name='password']")).await?.send_keys(password).await?;
    c.find(Locator::Css("input[name='btnG']")).await?.click().await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Logged in");
    let verify_mail = c.find(Locator::LinkText("Verify your Rockstar Games Social Club emai...")).await?;
    let href = verify_mail.attr("href").await?.unwrap().to_string();
    c.goto(&href).await?;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let verify_url = c.find(Locator::Css("a[target='_blank']")).await?.text().await?;

    c.goto(&verify_url).await?;
    
    let html = c.source().await?;

    if html.contains("Puede que tu cuenta ya haya sido verificada") {
        println!("Account {} is already verified", email);
    } else if html.contains("Tu correo electrónico se ha verificado") {
        println!("Account {} is verified", email);
    } else {
        println!("Error with account {}", email);
    }

    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    c.close().await?;

    Ok("code.to_string()".to_string())
}