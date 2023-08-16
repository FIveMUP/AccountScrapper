extern crate serde_json;
use serde_json::Value;

use std::env;
use std::path::Path;
use std::future::Future;
use std::pin::Pin;
use std::io::Read;
use tokio::signal::ctrl_c;
use winreg::enums::*;
use winreg::RegKey;

use screenshots::Screen;
use std::{io::Write, collections::HashMap, sync::Arc};
use base64::{Engine as _, engine::general_purpose};
use std::fs;
use tokio::{sync::RwLock, sync::Mutex};
use winapi::{
    shared::windef::POINT,
    um::{
        wingdi::GetPixel,
        winuser::{
            ClientToScreen, FindWindowW, GetCursorPos, GetDC,
            GetForegroundWindow, GetSystemMetrics, ReleaseDC, ScreenToClient, SendInput,
            SetForegroundWindow, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_UNICODE,
            MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
            MOUSEINPUT, SM_CXSCREEN, SM_CYSCREEN,
        },
    },
};
use regex::Regex;
mod mail_verifier;

struct Account {
    email: String,
    password: String,
}

struct  VerifyCaptchaMessage {
    color: u32,
    message: &'static str,
}

struct VerifyCaptcha {
    position: POINT,
    validation: VerifyCaptchaMessage,
}

struct Offsets {
    window_name: &'static str,
    email: POINT,
    password: POINT,
    sign_in: POINT,
    verify_captcha_btn: POINT,
    verify_mail_input: POINT,
    verify_mail_btn: POINT,
    verify_captcha_buttons: [POINT; 6],
    verify_captcha_messages: [VerifyCaptcha; 6],
}

static ROCKSTAR_OFFSETS: Offsets = Offsets {
    window_name: "Rockstar Games - Social Club",
    email: POINT { x: 350, y: 245 },
    password: POINT { x: 335, y: 300 },
    sign_in: POINT { x: 845, y: 380 },
    verify_captcha_btn: POINT { x: 630, y: 397 },
    verify_mail_input: POINT { x: 333, y: 285 },
    verify_mail_btn: POINT { x: 845, y: 350 },
    verify_captcha_buttons: [POINT { x: 535, y: 275 }, POINT { x: 635, y: 275 }, POINT { x: 735, y: 275 }, POINT { x: 535, y: 375 }, POINT { x: 635, y: 375 }, POINT { x: 735, y: 375 }],
    verify_captcha_messages: [
        VerifyCaptcha {
            position: POINT { x: 527, y: 256 }, 
            validation: VerifyCaptchaMessage { color: 3387392, message: "try_again_click" },
        },
        VerifyCaptcha {
            position: POINT { x: 682, y: 329 }, 
            validation: VerifyCaptchaMessage { color: 0, message: "try_again" },
        },
        VerifyCaptcha {
            position: POINT { x: 657, y: 310 }, 
            validation: VerifyCaptchaMessage { color: 3048749, message: "solved" },
        },
        VerifyCaptcha {
            position: POINT { x: 303, y: 440 }, 
            validation: VerifyCaptchaMessage { color: 526525, message: "verify_mail" },
        },
        VerifyCaptcha {
            position: POINT { x: 303, y: 440 }, 
            validation: VerifyCaptchaMessage { color: 666, message: "window_not_found" },
        },
        VerifyCaptcha {
            position: POINT { x: 897, y: 362 }, 
            validation: VerifyCaptchaMessage { color: 16777215, message: "verify_mail_code" },
        },

    ],

};

async fn keyboard_write(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut inputs: Vec<INPUT> = Vec::new();

    for c in text.chars() {
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: unsafe { std::mem::zeroed() },
        };

        unsafe {
            *input.u.ki_mut() = KEYBDINPUT {
                wVk: 0,
                wScan: c as u16,
                dwFlags: KEYEVENTF_UNICODE,
                dwExtraInfo: 0,
                time: 0,
            };
        }

        inputs.push(input);
    }

    for mut input in inputs {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        unsafe {
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
    }

    Ok(())
}

fn _print_mouse_position_relative_to_window(window_name: &str) -> POINT {
    let wide_name: Vec<u16> = window_name.encode_utf16().collect();
    let wide_name_null_terminated = [&wide_name[..], &[0u16][..]].concat();

    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide_name_null_terminated.as_ptr()) };
    if hwnd.is_null() {
        eprintln!("Window not found!");
        panic!("Window not found!");
    }

    let mut point = unsafe {
        let mut point = std::mem::zeroed();
        GetCursorPos(&mut point);
        point
    };

    unsafe { ScreenToClient(hwnd, &mut point) };

    let real_point = unsafe {
        let mut point = std::mem::zeroed();
        GetCursorPos(&mut point);
        point
    };

    println!(
        "Mouse position relative to window: ({}, {}), real position: ({}, {})",
        point.x, point.y,
        real_point.x, real_point.y
    );

    POINT {
        x: point.x,
        y: point.y,
    }
}

async fn get_pixel_color(
    window_name: &str,
    x: i32,
    y: i32,
) -> Result<u32, Box<dyn std::error::Error>> {
    let wide_name: Vec<u16> = window_name.encode_utf16().collect();
    let _wide_name_null_terminated = [&wide_name[..], &[0u16][..]].concat();

    let hwnd = unsafe { FindWindowW(std::ptr::null(), _wide_name_null_terminated.as_ptr()) };
    if hwnd.is_null() {
        eprintln!("Window not found!");
        return Ok(666 as u32);
    }

    let mut point = POINT { x, y };
    unsafe { ClientToScreen(hwnd, &mut point) };

    let hdc = unsafe { GetDC(hwnd) };

    // convert x,y to screen coordinates
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let x_scaled = (point.x as f64 / width as f64 * 65535.0) as i32;
    let y_scaled = (point.y as f64 / height as f64 * 65535.0) as i32;

    let color = unsafe { GetPixel(hdc, x, y) };
    // move mouse ti the get pixel position
    let mouse_input_move = MOUSEINPUT {
        dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
        dx: x_scaled,
        dy: y_scaled,
        mouseData: 0,
        dwExtraInfo: 0,
        time: 0,
    };

    let mut input_move = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe { std::mem::zeroed() },
    };

    unsafe {
        *input_move.u.mi_mut() = mouse_input_move;
        SendInput(1, &mut input_move, std::mem::size_of::<INPUT>() as i32);
    }

    unsafe { ReleaseDC(hwnd, hdc) };

    if color == -1i32 as u32 {
        eprintln!("Failed to get pixel color.");
        return Err("Failed to get pixel color.".into());
    }

    // let r = (color & 0x000000FF) as u8;
    // let g = ((color & 0x0000FF00) >> 8) as u8;
    // let b = ((color & 0x00FF0000) >> 16) as u8;


    Ok(color)
}

async fn ghost_click(window_name: &str, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
    let wide_name: Vec<u16> = window_name.encode_utf16().collect();
    let wide_name_null_terminated = [&wide_name[..], &[0u16][..]].concat();

    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide_name_null_terminated.as_ptr()) };
    if hwnd.is_null() {
        println!("Window not found!");
    }

    unsafe {
        let foreground_window = GetForegroundWindow();
        if foreground_window != hwnd {
            SetForegroundWindow(hwnd);
        }
    }

    let mut client_point = POINT { x, y };
    unsafe { ClientToScreen(hwnd, &mut client_point) };

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let x_scaled = (client_point.x as f64 / width as f64 * 65535.0) as i32;
    let y_scaled = (client_point.y as f64 / height as f64 * 65535.0) as i32;

    let first_pos = unsafe {
        let mut point = std::mem::zeroed();
        GetCursorPos(&mut point);
        point
    };

    // get coordinates of the mouse based on the screen size

    let first_pos_x = (first_pos.x as f64 / width as f64 * 65535.0) as i32;
    let first_pos_y = (first_pos.y as f64 / height as f64 * 65535.0) as i32;

    let mouse_input_move = MOUSEINPUT {
        dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
        dx: x_scaled,
        dy: y_scaled,
        mouseData: 0,
        dwExtraInfo: 0,
        time: 0,
    };

    let mouse_input_move_back = MOUSEINPUT {
        dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
        dx: first_pos_x,
        dy: first_pos_y,
        mouseData: 0,
        dwExtraInfo: 0,
        time: 0,
    };

    let mouse_input_down = MOUSEINPUT {
        dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTDOWN,
        dx: x_scaled,
        dy: y_scaled,
        mouseData: 0,
        dwExtraInfo: 0,
        time: 0,
    };

    let mouse_input_up = MOUSEINPUT {
        dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTUP,
        dx: x_scaled,
        dy: y_scaled,
        mouseData: 0,
        dwExtraInfo: 0,
        time: 0,
    };

    let mut input_move = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe { std::mem::zeroed() },
    };

    let mut input_down = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe { std::mem::zeroed() },
    };

    let mut input_up = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe { std::mem::zeroed() },
    };

    let mut input_move_back = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe { std::mem::zeroed() },
    };

    unsafe {
        *input_move.u.mi_mut() = mouse_input_move;
        *input_down.u.mi_mut() = mouse_input_down;
        *input_up.u.mi_mut() = mouse_input_up;
        *input_move_back.u.mi_mut() = mouse_input_move_back;

        SendInput(1, &mut input_move, std::mem::size_of::<INPUT>() as i32);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);
        SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

async fn captcha_click(captcha_array: &Vec<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let mut captcha_index = 0;
    for captcha in captcha_array {
        if captcha.as_bool().unwrap() {
            break;
        }
        captcha_index += 1;
    }

    println!("Captcha index: {}", captcha_index);

    let captcha_button_pos = ROCKSTAR_OFFSETS.verify_captcha_buttons[captcha_index];

    ghost_click(
        "Rockstar Games - Social Club",
        captcha_button_pos.x,
        captcha_button_pos.y,
    ).await?;

    Ok(())
}

async fn make_async_loop_fn_with_retries<F, Fut>(
    _fn: F,
    ms: u64,
    retries: u8,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let mut retries_count = 0;
    loop {
        match _fn().await {
            Ok(()) => return Ok(()),
            Err(_) if retries_count >= retries => return Err("Max retries reached".into()),
            Err(_) => {
                retries_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            }
        }
    }
}

#[derive(Clone)]
struct HWIDInfo {
    machine_hash_index: String,
    entitlement_id: String,
}

async fn update_account_data(mail: &str, status: &str, hwid_info: HWIDInfo) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()?;

    let pc_name = env::var("USERNAME").unwrap();

    let params = [
        ("mail", mail),
        ("status", status),
        ("machineHash", hwid_info.machine_hash_index.as_str()),
        ("entitlementId", hwid_info.entitlement_id.as_str()),
        ("pc_name", pc_name.as_str()),
        ("auth_token", "califatogang")
    ];

    let res = client.post("http://127.0.0.1:3001/api/accountStock/pending/update")
        .query(&params)
        .send()
        .await?;

    if res.status().as_u16() != 200 {
        println!("{}", res.text().await?);
        return Err("Error updating account data".into());
    } else {
        println!("Account data updated on DB!");
    }

    Ok(())
}

async fn hook_machine_hash() -> Result<HWIDInfo, Box<dyn std::error::Error>> {
    println!("Starting Hooking into GTA5.exe");
    #[cfg(windows)]
    if let Ok(p) = memflex::external::open_process_by_name(
        "_GTAProcess.exe",
        false,
        memflex::types::win::PROCESS_ALL_ACCESS,
    ) {
        println!("p: {:?}", p.name());

        let module = p.find_module("XAudio2_8.dll");
        
        if module.is_err() {
            return Err("XAudio2_8.dll not found".into());
        }

        let module = module.unwrap();

        println!("module: {:?}", module);

        let buffer_size = 64 * 1024;
        let mut buffer = vec![0u8; buffer_size];

        let start_address = module.base as usize;
        let end_address = module.base as usize + 8 * 1024 * 1024 * 1204;

        let re_machine = Regex::new(r"machineHashIndex=([^&]+)").unwrap();
        let re_entitlement = Regex::new(r"entitlementId=([^&]*)").unwrap();

        println!(
            "Starting exploring from 0x{:?} to 0x{:?} 8GB",
            start_address, end_address
        );

        std::io::stdout().flush().unwrap();

        let mut founded_data = HWIDInfo {
            machine_hash_index: "".to_string(),
            entitlement_id: "".to_string(),
        };

        for address in (start_address..end_address).step_by(buffer.len()) {
            if p.read_buf(address, &mut buffer).is_ok() {
                let result_string = String::from_utf8_lossy(&buffer);

                if let Some(index) = result_string.find("machineHash") {
                    let relevant_part = &result_string[index..];
                    if let Some(captures) = re_machine.captures(relevant_part) {
                        let machine_hash_index = captures.get(1).unwrap().as_str();
                        if machine_hash_index.len() > 44 {
                            continue;
                        }
                        print!("\x1B[s\nMachine hash index found: {:?} at 0x{:x}", machine_hash_index, address);
                        print!("\x1B[u[Checking: 0x{:x}]\r", address);
                        std::io::stdout().flush().unwrap();
                        founded_data.machine_hash_index = machine_hash_index.to_string();
                    }
                }

                if let Some(index) = result_string.find("entitlementId") {
                    let relevant_part = &result_string[index..];
                    if let Some(captures) = re_entitlement.captures(relevant_part) {
                        let entitlement_id = captures.get(1).unwrap().as_str();
                        if entitlement_id.len() > 35 {
                            continue;
                        }
                        print!("\x1B[s\nEntitlement id found: {:?} at 0x{:x}", entitlement_id, address);
                        print!("\x1B[u[Checking: 0x{:x}]\r", address);
                        std::io::stdout().flush().unwrap();
                        founded_data.entitlement_id = entitlement_id.to_string();
                    }
                }

                print!("[Checking: 0x{:x}]\r", address);
                std::io::stdout().flush().unwrap();

                if !founded_data.machine_hash_index.is_empty() && !founded_data.entitlement_id.is_empty() && founded_data.machine_hash_index.len() <= 44 && founded_data.entitlement_id.len() <= 35 {
                    println!("\n\nFound machine hash index: {:?} and entitlement id: {:?} at 0x{:x}", founded_data.machine_hash_index, founded_data.entitlement_id, address);
                    return Ok(founded_data);
                }
            }
        }

        println!("\n\n");
    } else {
        println!("Process not found");
    }

    Err("Query failed".into())
}

async fn get_captcha_message() -> Result<String, Box<dyn std::error::Error>> {    
    let final_message = RwLock::new(String::new());
    let result = make_async_loop_fn_with_retries(
            || async {

            for message in &ROCKSTAR_OFFSETS.verify_captcha_messages {
                let position = message.position;
                let color = message.validation.color;
                let message = message.validation.message;
                
                let _pixel_color = get_pixel_color(ROCKSTAR_OFFSETS.window_name, position.x, position.y).await?;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let pixel_color = get_pixel_color(ROCKSTAR_OFFSETS.window_name, position.x, position.y).await?;
                
                println!("Checking message: {:?}", message);

                if pixel_color == color {
                    final_message.write().await.push_str(&message);
                    return Ok(());
                }
            }

            if final_message.read().await.len() > 0 {
                return Ok(());
            } else {
                return Err("Captcha message not found".into());
            }
            
        },
        10,
        10,
    )
    .await;

    if result.is_err() {
        println!("Info: {:?}", result);
    } else if final_message.read().await.len() > 0 {
        return Ok(final_message.read().await.clone());
    }

    Ok("No message found".into())
}

fn solve_captcha() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>>>> {
    Box::pin(async {
        let screens = Screen::all().unwrap();
        let my_screen: &Screen = &screens[0];
        let image = my_screen.capture_area(805, 380, 305, 240).unwrap();
        let buffer = image.to_png(None).unwrap();
        fs::write(
            format!("{}-2.png", my_screen.display_info.id),
            buffer.clone(),
        )
        .unwrap();
        let base64 = general_purpose::STANDARD.encode(&buffer);

        let mut post_params: HashMap<&str, Value> = HashMap::new();

        post_params.insert("key", Value::String("sub_1NFmnDCRwBwvt6ptOZH8VdJn".to_string()));
        post_params.insert("type", Value::String("funcaptcha".to_string()));
        post_params.insert("task", Value::String("Pick the image that is the correct way up".to_string()));
        post_params.insert("image_data", Value::Array(vec![Value::String(base64)]));
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537")
            .build()?;
        
        let res = client.post("https://api.nopecha.com/")
            .json(&post_params)
            .send()
            .await;

        if res.is_err() {
            return solve_captcha().await;
        } 
        
        let res = res.unwrap();
        let parsed_response: Value = serde_json::from_str(&res.text().await?).unwrap();
        let solving_id = parsed_response["data"].as_str().unwrap();

        if solving_id.len() == 64 {
            println!("Captcha can be solved, starting solving!: {}", solving_id);
        } else {
            panic!("Captcha not solved!");
        }

        let get_params = [
            ("key", "sub_1NFmnDCRwBwvt6ptOZH8VdJn"),
            ("id", solving_id)
        ];


        let res = client.get("https://api.nopecha.com/")
            .query(&get_params)
            .send()
            .await;

        if res.is_err() {
            return solve_captcha().await;
        } 

        let res = res.unwrap();
        let response = res.text().await?;


        let parsed_response: Value = serde_json::from_str(&response).unwrap();
        let data: &Value = &parsed_response["data"];

        if !data.is_array() {
            return solve_captcha().await;
        }

        let data = data.as_array().unwrap();

        println!("Response of solved captcha: {:?}", data);

        captcha_click(data).await?;

        let captcha_message = get_captcha_message().await?;

        println!("Captcha message: {}", captcha_message);

        if captcha_message == "try_again_click" {
            println!("Captcha failed, trying again...");
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            ghost_click(
                ROCKSTAR_OFFSETS.window_name,
                ROCKSTAR_OFFSETS.verify_captcha_btn.x,
                ROCKSTAR_OFFSETS.verify_captcha_btn.y,
            )
            .await?;
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            return solve_captcha().await;
        } else if captcha_message == "try_again" {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            return solve_captcha().await;
        } else if captcha_message == "solved" {
            println!("Captcha solved!");
            return Ok(());
        } else {
            return Ok(());
        }
    })
}

async fn retrieve_account_data(account: Account) -> Result<HWIDInfo, Box<dyn std::error::Error>> {

    let final_hwid = Arc::new(RwLock::new(HWIDInfo {
        machine_hash_index: "".to_string(),
        entitlement_id: "".to_string(),
    }));

    ghost_click(
        ROCKSTAR_OFFSETS.window_name,
        ROCKSTAR_OFFSETS.email.x,
        ROCKSTAR_OFFSETS.email.y,
    )
    .await?;
    keyboard_write(&account.email).await?;
    ghost_click(
        ROCKSTAR_OFFSETS.window_name,
        ROCKSTAR_OFFSETS.password.x,
        ROCKSTAR_OFFSETS.password.y,
    )
    .await?;
    keyboard_write(&account.password).await?;
    ghost_click(
        ROCKSTAR_OFFSETS.window_name,
        ROCKSTAR_OFFSETS.sign_in.x,
        ROCKSTAR_OFFSETS.sign_in.y,
    )
    .await?;

    let captcha_found = make_async_loop_fn_with_retries(
        || async {
            println!("Trying to find captcha button...");

            let color = get_pixel_color(
                ROCKSTAR_OFFSETS.window_name,
                ROCKSTAR_OFFSETS.verify_captcha_btn.x,
                ROCKSTAR_OFFSETS.verify_captcha_btn.y,
            )
            .await?;

            if color == 1683451 {
                println!("Captcha button found!");
                return Ok(());
            } else {
                let verify_mail_offset = &ROCKSTAR_OFFSETS.verify_captcha_messages[5];
                println!("Captcha button not found!");
                let color = get_pixel_color(
                    ROCKSTAR_OFFSETS.window_name,
                    verify_mail_offset.position.x,
                    verify_mail_offset.position.y
                ).await?;

                println!("Color: {}, message: {}", color, verify_mail_offset.validation.message);

                if color == verify_mail_offset.validation.color {
                    println!("Needs to verify mail with code!");
                    tokio::time::sleep(std::time::Duration::from_millis(8500)).await;
                    let verification_code = mail_verifier::get_mail_code(&account.email, &account.password).await?;
                    
                    println!("Verification code: {:?}", verification_code);
                    
                    
                    ghost_click(
                        ROCKSTAR_OFFSETS.window_name,
                        ROCKSTAR_OFFSETS.verify_mail_input.x,
                        ROCKSTAR_OFFSETS.verify_mail_input.y,
                    )
                    .await?; 
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    keyboard_write(&verification_code).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    ghost_click(
                        ROCKSTAR_OFFSETS.window_name,
                        ROCKSTAR_OFFSETS.verify_mail_btn.x,
                        ROCKSTAR_OFFSETS.verify_mail_btn.y,
                    ).await?;

                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    ghost_click(
                        ROCKSTAR_OFFSETS.window_name,
                        ROCKSTAR_OFFSETS.sign_in.x,
                        ROCKSTAR_OFFSETS.sign_in.y,
                    ).await?;

                    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

                    return Ok(());
                }
                

                return Err("Captcha button not found!".into());
            }
        },
        200,
        25,
    )
    .await;

    if captcha_found.is_err() {
        println!("Captcha button not found!, trying to check if captcha has been skipped...");
    
        let result = make_async_loop_fn_with_retries(
            || async {
                let hwid_info = hook_machine_hash().await?;
    
                if hwid_info.machine_hash_index.len() >= 31 && hwid_info.entitlement_id.len() >= 31 {
                    println!("Machine hash found!");
                    println!("Machine hash: {}  EntitlementID: {}", hwid_info.machine_hash_index, hwid_info.entitlement_id);
                    final_hwid.write().await.machine_hash_index = hwid_info.machine_hash_index;
                    final_hwid.write().await.entitlement_id = hwid_info.entitlement_id;
                    return Ok(());
                } else {
                    println!("Machine hash not found!");
                    return Err("Machine hash not found!".into());
                }
            },
            2000,
            5,
        )
        .await;

        if result.is_err() {
            println!("Error hooking machine hash retries for account: {}", account.email);
        }

        let hwid_info = final_hwid.read().await.clone();
        return Ok(hwid_info);
    }

    ghost_click(
        ROCKSTAR_OFFSETS.window_name,
        ROCKSTAR_OFFSETS.verify_captcha_btn.x,
        ROCKSTAR_OFFSETS.verify_captcha_btn.y,
    )
    .await?;

    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
    
    solve_captcha().await?;

    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    let captcha_message = get_captcha_message().await?;

    if captcha_message == "verify_mail_code" {
        println!("Needs to verify mail with code!");
        tokio::time::sleep(std::time::Duration::from_millis(8500)).await;
        let verification_code: String = mail_verifier::get_mail_code(&account.email, &account.password).await?;
        
        println!("Verification code: {:?}", verification_code);
        
        
        ghost_click(
            ROCKSTAR_OFFSETS.window_name,
            ROCKSTAR_OFFSETS.verify_mail_input.x,
            ROCKSTAR_OFFSETS.verify_mail_input.y,
        )
        .await?; 
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        keyboard_write(&verification_code).await?;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        ghost_click(
            ROCKSTAR_OFFSETS.window_name,
            ROCKSTAR_OFFSETS.verify_mail_btn.x,
            ROCKSTAR_OFFSETS.verify_mail_btn.y,
        ).await?;

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        ghost_click(
            ROCKSTAR_OFFSETS.window_name,
            ROCKSTAR_OFFSETS.sign_in.x,
            ROCKSTAR_OFFSETS.sign_in.y,
        ).await?;

        tokio::time::sleep(std::time::Duration::from_millis(4500)).await;
        
        ghost_click(
            ROCKSTAR_OFFSETS.window_name,
            ROCKSTAR_OFFSETS.verify_captcha_btn.x,
            ROCKSTAR_OFFSETS.verify_captcha_btn.y,
        )
        .await?;
    
        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        
        solve_captcha().await?;
    
    }

    println!("Captcha solved! i think :)");

    tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
    
    let captcha_message = get_captcha_message().await?;

    if captcha_message == "verify_mail" {
        println!("Needs to verify the mail!");
    }

    let result = make_async_loop_fn_with_retries(
        || async {
            let hwid_info = hook_machine_hash().await?;

            if hwid_info.machine_hash_index.len() >= 31 && hwid_info.entitlement_id.len() >= 31 {
                println!("Machine hash found!");
                println!("Machine hash: {}  EntitlementID: {}", hwid_info.machine_hash_index, hwid_info.entitlement_id);
                final_hwid.write().await.machine_hash_index = hwid_info.machine_hash_index;
                final_hwid.write().await.entitlement_id = hwid_info.entitlement_id;
                return Ok(());
            } else {
                println!("Machine hash not found!");
                return Err("Machine hash not found!".into());
            }
        },
        2000,
        5,
    )
    .await;

    if result.is_err() {
        println!("Error hooking machine hash retries for account: {}", account.email);
    }

    let hwid_info = final_hwid.read().await.clone();
    Ok(hwid_info)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // loop {
    //     let client_point = _print_mouse_position_relative_to_window(ROCKSTAR_OFFSETS.window_name);
    //     let pixel_color = get_pixel_color(ROCKSTAR_OFFSETS.window_name, client_point.x, client_point.y).await.unwrap_or_else(|_| 0);
    //     println!("Pixel color: {}", pixel_color);
    //     println!("Client point: x: {} y: {}", client_point.x, client_point.y);
    //     tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // }

    // let system = RegKey::predef(HKEY_LOCAL_MACHINE)
    // .open_subkey("HARDWARE\\DESCRIPTION\\System")?;
    // for (name, value) in system.enum_values().map(|x| x.unwrap()) {
    //     println!("{} = {:?}", name, value);
    // }

    let pc_username = env::var("USERNAME").unwrap();
    let current_checking_mail = Arc::new(RwLock::new(String::new()));

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537")
        .build()?;

    let cloned_mail = current_checking_mail.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Ctrl-C received, press again to exit");
        tokio::signal::ctrl_c().await.unwrap();
        
        // Clone mail and read it
        let mail = cloned_mail.read().await.clone();
        update_account_data(&mail, "error", HWIDInfo { machine_hash_index: "".into(), entitlement_id: "".into() }).await.unwrap();

        println!("Exiting...");
        std::process::exit(0);
    });

    loop {
        println!("[ :) ] Checking Accounts as: {}", pc_username);

        let params = [
            ("pc_name", pc_username.as_str()),
            ("auth_token", "califatogang")
        ];

        let check_data = client.get("http://127.0.0.1:3001/api/accountStock/pending/getNext")
            .query(&params)
            .send()
            .await;

        if check_data.is_err() {
            println!("Error getting accounts to check: {:?}", check_data);
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
            continue;
        }

        let check_data = check_data.unwrap();
        
        if check_data.status().as_u16() != 200 {
            println!("Error getting accounts to check: {:?}", check_data);
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
            continue;
        }

        let check_data_json: Value = serde_json::from_str(&check_data.text().await?).unwrap();

        let email = check_data_json["email"].as_str().unwrap();
        let password = check_data_json["password"].as_str().unwrap();
        let message = check_data_json["message"].as_str().unwrap();
        println!("{}", message);

        current_checking_mail.write().await.push_str(&email);
        
        tokio::process::Command::new("taskkill")
        .args(&["/F", "/IM", "FiveM.exe"])
        .spawn()
        .expect("Failed to kill FiveM.exe");

        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        
        let digital_entitlements_path = format!("{}\\AppData\\Local\\DigitalEntitlements", env::var("USERPROFILE").unwrap());
        if Path::new(&digital_entitlements_path.clone()).exists() {
            fs::remove_dir_all(digital_entitlements_path.clone())?;
        }

        let fivem_path = format!("{}\\AppData\\Local\\FiveM\\FiveM.exe", env::var("USERPROFILE").unwrap());
        println!("FiveM path: {}", fivem_path);
        tokio::process::Command::new(fivem_path)
            .spawn()
            .expect("Failed to start FiveM.exe");


        while !Path::new(&digital_entitlements_path).exists() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        println!("FiveM ready to be hooked!");

        tokio::time::sleep(std::time::Duration::from_millis(6000)).await;

        let hwid_info = retrieve_account_data(Account {
            email: email.to_string(), password: password.to_string(),
        }).await;

        if hwid_info.is_err() {
            println!("Error retrieving account: {}", email);
            update_account_data(&email, "error_getting_hwid", HWIDInfo { machine_hash_index: "".into(), entitlement_id: "".into() }).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
            continue;
        }

        let hwid_info = hwid_info.unwrap();

        tokio::process::Command::new("taskkill")
            .args(&["/F", "/IM", "FiveM.exe"])
            .spawn()
            .expect("Failed to kill FiveM.exe");

        if hwid_info.machine_hash_index.len() < 31 || hwid_info.entitlement_id.len() < 31 {
            println!("Account {} failed!", email);
            update_account_data(&email, "error_getting_hwid_too_long", HWIDInfo { machine_hash_index: "".into(), entitlement_id: "".into() }).await.unwrap();
            continue;
        }

        update_account_data(&email, "checked", hwid_info.clone()).await.unwrap();

        let check_string = format!("{}:{}:{}:{}", hwid_info.machine_hash_index, hwid_info.entitlement_id, email, password);
        println!("Account {} checked!", check_string);
        
        println!("Machine hash: {}  EntitlementID: {} Email: {}  Password: {}", hwid_info.machine_hash_index, hwid_info.entitlement_id, email, password);
    }
}
