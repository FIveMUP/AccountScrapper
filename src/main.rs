use winapi::{
    shared::windef::POINT,
    um::{
        wingdi::{GetPixel, RGB},
        winuser::{
            ClientToScreen, FindWindowW, GetCursorPos, GetDC, GetDesktopWindow,
            GetForegroundWindow, GetSystemMetrics, ReleaseDC, ScreenToClient, SendInput,
            SetForegroundWindow, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_UNICODE,
            MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
            MOUSEINPUT, SM_CXSCREEN, SM_CYSCREEN,
        },
    },
};

struct Account {
    email: String,
    password: String,
}
struct Offsets {
    email: POINT,
    password: POINT,
    sign_in: POINT,
    verify_captcha_btn: POINT,
}

static ROCKSTAR_OFFSETS: Offsets = Offsets {
    email: POINT { x: 350, y: 245 },
    password: POINT { x: 335, y: 300 },
    sign_in: POINT { x: 845, y: 380 },
    verify_captcha_btn: POINT { x: 630, y: 397 },
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

    println!(
        "Mouse position relative to window: ({}, {})",
        point.x, point.y
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
        panic!("Window not found!");
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

    println!(
        "Mouse position relative to screen: ({}, {})",
        point.x, point.y
    );

    unsafe {
        *input_move.u.mi_mut() = mouse_input_move;
        SendInput(1, &mut input_move, std::mem::size_of::<INPUT>() as i32);
    }

    unsafe { ReleaseDC(hwnd, hdc) };

    if color == -1i32 as u32 {
        eprintln!("Failed to get pixel color.");
        return Err("Failed to get pixel color.".into());
    }

    let r = (color & 0x000000FF) as u8;
    let g = ((color & 0x0000FF00) >> 8) as u8;
    let b = ((color & 0x00FF0000) >> 16) as u8;

    println!("R: {}, G: {}, B: {}", r, g, b);

    Ok(color)
}

async fn ghost_click(window_name: &str, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
    let wide_name: Vec<u16> = window_name.encode_utf16().collect();
    let wide_name_null_terminated = [&wide_name[..], &[0u16][..]].concat();

    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide_name_null_terminated.as_ptr()) };
    if hwnd.is_null() {
        eprintln!("Window not found!");
        panic!("Window not found!");
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
        // SendInput(1, &mut input_move_back, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

async fn make_async_loop_fn_with_retries<F, E>(
    mut func: F,
    ms: u64,
    mut retries: u8,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() -> Result<(), E>,
{
    while retries > 0 {
        if func().is_ok() {
            return Ok(());
        }
        retries -= 1;
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "retries exhausted",
    )))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window_name = "Rockstar Games - Social Club";
    // loop {
    //     let client_point = _print_mouse_position_relative_to_window(window_name);
    //     println!(
    //         "Mouse position relative to screen: ({}, {})",
    //         client_point.x, client_point.y
    //     );
    //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // }

    let account = Account {
        email: "cristian124421@gmail.com".to_string(),
        password: "Lokesea124!".to_string(),
    };

    ghost_click(
        window_name,
        ROCKSTAR_OFFSETS.email.x,
        ROCKSTAR_OFFSETS.email.y,
    )
    .await?;
    keyboard_write(&account.email).await?;
    ghost_click(
        window_name,
        ROCKSTAR_OFFSETS.password.x,
        ROCKSTAR_OFFSETS.password.y,
    )
    .await?;
    keyboard_write(&account.password).await?;
    ghost_click(
        window_name,
        ROCKSTAR_OFFSETS.sign_in.x,
        ROCKSTAR_OFFSETS.sign_in.y,
    )
    .await?;

    make_async_loop_fn_with_retries(
        || {
            println!("Trying to find captcha button...");

            if false {
                Ok(())
            } else {
                Err("Captcha button not found")
            }
        },
        150,
        10,
    )
    .await?;

    Ok(())
}
