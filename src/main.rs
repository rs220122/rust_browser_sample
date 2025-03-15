#![no_std]
#![no_main]

extern crate alloc;

use crate::alloc::string::ToString;
use noli::prelude::*;
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
    let response = HttpResponse::new(TEST_HTTP_RESPONSE.to_string())
        .expect("failed to parse response");
    let page = browser.borrow().current_page();
    let dom_string = page.borrow_mut().receive_response(response);

    for log in dom_string.lines() {
        println!("{}", log);
    }
    0
}

entry_point!(main);
