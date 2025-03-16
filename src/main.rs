#![no_std]
#![no_main]

extern crate alloc;

use crate::alloc::string::ToString;
use alloc::rc::Rc;
use core::cell::RefCell;
use noli::prelude::*;
use ui_wasabi::app::WasabiUI;

use saba_core::browser::Browser;
use saba_core::http::HttpResponse;
use saba_core::renderer::css::parser::CssParser;
use saba_core::renderer::css::token::CssTokenizer;
use saba_core::renderer::dom::api::get_style_content;
use saba_core::renderer::dom::window::Window;
use saba_core::renderer::html::parser::HtmlParser;
use saba_core::renderer::html::token::HtmlTokenizer;

static TEST_HTTP_RESPONSE: &str = r#"HTTP/1.1 200 OK
Data: xx xx xx

<html>
<head>
<style>
  .hidden {
    display: none;
  }
</style>
</head>
<body>
    <h1 id="title">H1 title</h1>
    <h2 class="class">H2 title</h2>
    <p> Test Text.</p>
    <p>
        <a href="example.com">link1</a>
        <a href="example.com">link2</a>
    </p>
</body>
</html>
"#;

fn main() -> u64 {
    let browser = Browser::new();

    let ui = Rc::new(RefCell::new(WasabiUI::new(browser)));

    // アプリを起動
    match ui.borrow_mut().start() {
        Ok(_) => {}
        Err(e) => {
            println!("browser fails to start: {:?}", e);
            return 1;
        }
    };
    0
}

entry_point!(main);
