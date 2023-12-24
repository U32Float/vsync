use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use dashmap::DashMap;

// ----------------------------------------------------------------------------

pub type DisplayID = u32;

pub struct VSync {}

impl VSync {
    pub fn time_until_next_frame(display: DisplayID) -> Duration {
        unimplemented!()
    }

    pub fn wait_until_next_frame(display: DisplayID) {
        Self::display_link(display).wait_vsync();
    }
}

impl VSync {
    fn display_link(display: DisplayID) -> DisplayLink {
        static DISPLAY_LINKS: OnceLock<DashMap<DisplayID, DisplayLink>> = OnceLock::new();

        DISPLAY_LINKS
            .get_or_init(|| DashMap::new())
            .get(&display)
            .map(|l| l.clone())
            .or_else(|| {
                let link = DisplayLink::new(display);
                DISPLAY_LINKS.get().unwrap().insert(display, link.clone());
                Some(link)
            })
            .unwrap()
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone)]
struct DisplayLink {
    next_frame: Arc<Mutex<Option<Duration>>>,
    /// Only used for keeping the display link alive.
    _wrapper: Arc<DisplayLinkWrapper>,
}

impl DisplayLink {
    pub fn new(display: DisplayID) -> Self {
        let mut link = display_link::DisplayLink::on_display(display, |t| {
            println!("{:?}", t);
        })
        .unwrap();
        let _ = link.resume();

        Self {
            next_frame: Arc::new(Mutex::new(None)),
            _wrapper: Arc::new(DisplayLinkWrapper::new(link)),
        }
    }

    pub fn wait_vsync(&self) {}
}

// ----------------------------------------------------------------------------

struct DisplayLinkWrapper {
    ptr: *mut display_link::DisplayLink,
}

unsafe impl Send for DisplayLinkWrapper {}
unsafe impl Sync for DisplayLinkWrapper {}

impl DisplayLinkWrapper {
    pub fn new(display_link: display_link::DisplayLink) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(display_link)),
        }
    }
}

impl Drop for DisplayLinkWrapper {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.ptr);
        }
    }
}
