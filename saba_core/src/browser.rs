use crate::renderer::page::Page;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

pub struct Browser {
    active_page_index: usize,
    pages: Vec<Rc<RefCell<Page>>>,
}

impl Browser {
    pub fn new() -> Rc<RefCell<Self>> {
        let mut page = Page::new();

        let browser = Rc::new(RefCell::new(Browser {
            active_page_index: 0,
            pages: Vec::new(),
        }));

        page.set_browser(Rc::downgrade(&browser));
        browser.borrow_mut().add_page(Rc::new(RefCell::new(page)));
        browser
    }

    fn add_page(&mut self, page: Rc<RefCell<Page>>) {
        self.pages.push(page);
    }

    pub fn current_page(&self) -> Rc<RefCell<Page>> {
        self.pages[self.active_page_index].clone()
    }
}
