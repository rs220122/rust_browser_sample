#![no_std]
#![no_main]

extern crate alloc;

use crate::alloc::string::String;
use crate::alloc::string::ToString;
use alloc::format;
use alloc::rc::Rc;
use core::cell::RefCell;
use net_wasabi::http::HttpClient;
use noli::*;
use saba_core::error::Error;
use saba_core::url::Url;
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

fn handle_url(url: String) -> Result<HttpResponse, Error> {
    // URLを解釈する
    let mut get_count = 0;
    let client = HttpClient::new();
    let mut response = None;

    let mut url = url;
    while true {
        let parsed_url = match Url::new(url.clone()).parse() {
            Ok(url) => url,
            Err(e) => {
                return Err(Error::UnexpectedInput(format!(
                    "input html is not supported: {:?}",
                    e,
                )));
            }
        };
        get_count += 1;

        // HTTPリクエストを送信する
        response = match client.get(
            parsed_url.host(),
            parsed_url.port().parse::<u16>().expect(&format!(
                "port number should be u16 but got {}",
                parsed_url.port()
            )),
            parsed_url.path(),
        ) {
            Ok(response) => {
                // リダイレクトが5回以上続いたら最後のレスポンスを返す
                // リダイレクトの場合はリダイレクト先のURLにリクエストを送る
                if get_count < 5 && response.status_code() == 302 {
                    let location = match response.header_value("Location") {
                        Ok(value) => value,
                        Err(_) => return Ok(response),
                    };
                    url = location;
                    continue;
                }
                Some(response)
            }
            Err(e) => {
                return Err(Error::Network(format!(
                    "failed to get http response: {:?}",
                    3
                )));
            }
        };
        break;
    }
    Ok(response.expect("response should not be None"))
}

fn main() -> u64 {
    let browser = Browser::new();

    let ui = Rc::new(RefCell::new(WasabiUI::new(browser)));

    // アプリを起動
    match ui.borrow_mut().start(handle_url) {
        Ok(_) => {}
        Err(e) => {
            println!("browser fails to start: {:?}", e);
            return 1;
        }
    };
    0
}

entry_point!(main);
