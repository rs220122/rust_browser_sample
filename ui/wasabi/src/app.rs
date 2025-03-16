use alloc::format;
use alloc::string::String;
use alloc::{rc::Rc, string::ToString};
use core::cell::RefCell;
use noli::error::Result as OsResult;
use noli::prelude::SystemApi;
use noli::println;
use noli::rect::Rect;
use noli::sys::api::MouseEvent;
use noli::sys::wasabi::Api;
use noli::window::StringSize;
use noli::window::Window;

use saba_core::browser::Browser;
use saba_core::error::Error;

use saba_core::constants::*;

use crate::cursor::Cursor;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
pub struct WasabiUI {
    browser: Rc<RefCell<Browser>>,
    // ユーザーが入力した文字を保持する
    input_url: String,
    input_mode: InputMode,
    // UIウィンドウィの管理を行う
    window: Window,
    cursor: Cursor,
}

impl WasabiUI {
    pub fn new(browser: Rc<RefCell<Browser>>) -> Self {
        Self {
            browser,
            input_url: String::new(),
            input_mode: InputMode::Normal,
            window: Window::new(
                "SaBA".to_string(),
                WHITE,
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .unwrap(),
            cursor: Cursor::new(),
        }
    }

    fn setup_toolbar(&mut self) -> OsResult<()> {
        // ツールバーの背景の矩形を描画
        // self.window.fill_rect(LIGHTGREY, 0, 0, WINDOW_WIDTH, TOOLBAR_HEIGHT)?;

        // ツールバーとコンテンツエリアーの境目の線を描画
        self.window.draw_line(
            GREY,
            0,
            TOOLBAR_HEIGHT,
            WINDOW_WIDTH - 1,
            TOOLBAR_HEIGHT,
        )?;
        self.window.draw_line(
            DARKGREY,
            0,
            TOOLBAR_HEIGHT + 1,
            WINDOW_WIDTH - 1,
            TOOLBAR_HEIGHT + 1,
        )?;

        // アドレスバーの横に"Address:"と表示
        self.window.draw_string(
            BLACK,
            5,
            5,
            "Address",
            StringSize::Medium,
            false,
        )?;

        // アドレスバーの矩形を描画
        let addressbar: u32 = 0xeef2f9;
        self.window.fill_rect(
            addressbar,
            70,
            2,
            WINDOW_WIDTH - 74,
            2 + ADDRESSBAR_HEIGHT,
        )?;

        // アドレスバーの影の線を描画
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), Error> {
        self.setup()?;
        self.run_app()?;
        Ok(())
    }

    fn setup(&mut self) -> Result<(), Error> {
        if let Err(error) = self.setup_toolbar() {
            return Err(Error::InvalidUI(format!(
                "failed to initialize a toolbar with error: {:?}",
                error
            )));
        }
        self.window.flush();
        Ok(())
    }

    fn run_app(&mut self) -> Result<(), Error> {
        loop {
            self.handle_mouse_input()?;
            self.handle_key_input()?;
        }
    }

    // マウスの入力を処理する
    fn handle_mouse_input(&mut self) -> Result<(), Error> {
        if let Some(MouseEvent { button, position }) =
            Api::get_mouse_cursor_info()
        {
            self.window.flush_area(self.cursor.rect());
            self.cursor.set_position(position.x, position.y);
            self.window.flush_area(self.cursor.rect());
            self.cursor.flush();

            // l: 左ボタンがクリックされた時にtrue
            // c: スクロールボタンがクリックされた時にtrue
            // r: 右ボタンがクリックされた時にtrue
            if button.l() || button.c() || button.r() {
                // 相対位置を計算する
                let relative_pos = (
                    position.x - WINDOW_INIT_X_POS,
                    position.y - WINDOW_INIT_Y_POS,
                );

                // ウィンドウの外をクリックされた時は何もしない
                if relative_pos.0 < 0
                    || relative_pos.0 > WINDOW_WIDTH
                    || relative_pos.1 < 0
                    || relative_pos.1 > WINDOW_HEIGHT
                {
                    println!(
                    "button clicked OUTSIZE  window: {button:?} {position:?}"
                );
                    return Ok(());
                }

                // ツールバーの範囲をクリックされた時は、InputMode=Editingにする
                if relative_pos.1 < TOOLBAR_HEIGHT + TITLE_BAR_HEIGHT
                    && relative_pos.1 >= TITLE_BAR_HEIGHT
                {
                    self.clear_address_bar()?;
                    self.input_url = String::new();
                    self.input_mode = InputMode::Editing;
                    println!("input mode: Editing");
                    return Ok(());
                }
                self.input_mode = InputMode::Normal;
            }
        }

        Ok(())
    }

    // ユーザーの入力を処理する
    fn handle_key_input(&mut self) -> Result<(), Error> {
        match self.input_mode {
            InputMode::Normal => {
                let _ = Api::read_key();
            }
            InputMode::Editing => {
                if let Some(c) = Api::read_key() {
                    if (c == 0x7F as char || c == 0x08 as char)
                        && !self.input_url.is_empty()
                    {
                        self.input_url.pop();
                    } else {
                        self.input_url.push(c);
                    }
                    self.update_address_bar()?;
                }
            }
        }
        Ok(())
    }

    // アドレスバーの内容を更新する
    fn update_address_bar(&mut self) -> Result<(), Error> {
        if self
            .window
            .fill_rect(WHITE, 72, 4, WINDOW_WIDTH - 76, ADDRESSBAR_HEIGHT - 2)
            .is_err()
        {
            return Err(Error::InvalidUI(
                "failed to clear an address bar".to_string(),
            ));
        }
        // input_Urlをアドレスバーに描画する
        if self
            .window
            .draw_string(
                BLACK,
                74,
                6,
                &self.input_url,
                StringSize::Medium,
                false,
            )
            .is_err()
        {
            return Err(Error::InvalidUI(
                "failed to update an address bar".to_string(),
            ));
        }

        // アドレスバーの部分の画面を更新する
        self.window.flush_area(
            Rect::new(
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS + TITLE_BAR_HEIGHT,
                WINDOW_WIDTH,
                TOOLBAR_HEIGHT,
            )
            .expect("failed to create a rect for the address bar"),
        );
        Ok(())
    }

    // 以前書かれたアドレスの内容を消す
    fn clear_address_bar(&mut self) -> Result<(), Error> {
        // アドレスバーを白く塗る
        if self
            .window
            .fill_rect(WHITE, 72, 4, WINDOW_WIDTH - 76, ADDRESSBAR_HEIGHT - 2)
            .is_err()
        {
            return Err(Error::InvalidUI(
                "failed to clear an address bar".to_string(),
            ));
        }
        // アドレスバーの部分の画面を更新する
        self.window.flush_area(
            Rect::new(
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS + TITLE_BAR_HEIGHT,
                WINDOW_WIDTH,
                TOOLBAR_HEIGHT,
            )
            .expect("failed to create a rect for the address bar"),
        );

        Ok(())
    }
}
