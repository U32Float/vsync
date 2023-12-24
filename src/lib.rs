use std::{
    sync::{Arc, OnceLock, RwLock},
    time::{Duration, SystemTime},
};

use dashmap::DashMap;

// ----------------------------------------------------------------------------

pub type DisplayID = u32;

pub fn time_until_next_frame(display: DisplayID) -> Duration {
    display_link(display).time_until_next_frame()
}

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

// ----------------------------------------------------------------------------

struct State {
    /// The time for the next frame in nanoseconds since UNIX_EPOCH.
    next_frame: Option<u128>,
    /// The time a single frame takes in nanoseconds.
    frame_time: Option<u64>,
}

#[derive(Clone)]
struct DisplayLink {
    state: Arc<RwLock<State>>,
    /// Only used for keeping the display link alive.
    _wrapper: Arc<DisplayLinkWrapper>,
}

impl DisplayLink {
    fn new(display: DisplayID) -> Self {
        let state = Arc::new(RwLock::new(State {
            next_frame: None,
            frame_time: None,
        }));

        let state_clone = state.clone();
        let mut link = display_link::DisplayLink::on_display(display, move |t| {
            let duration_since_epoch = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap();
            let timestamp_nanos = duration_since_epoch.as_nanos();
            let next_frame = timestamp_nanos + t.nanos_since_zero as u128;
            let prev = state_clone.read().unwrap().next_frame.clone();
            if let Some(prev) = prev {
                state_clone.write().unwrap().frame_time = Some((next_frame - prev) as u64)
            }
            state_clone.write().unwrap().next_frame = Some(next_frame);
        })
        .unwrap();
        let _ = link.resume();

        Self {
            state,
            _wrapper: Arc::new(DisplayLinkWrapper::new(link)),
        }
    }

    fn time_until_next_frame(&self) -> Duration {
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();

        let next_frame = self.state.read().unwrap().next_frame.clone();
        if let Some(next_frame) = next_frame {
            if next_frame > timestamp_nanos {
                return Duration::from_nanos((next_frame - timestamp_nanos) as u64);
            } else {
                if let Some(frame_time) = self.state.read().unwrap().frame_time.clone() {
                    return Duration::from_nanos(
                        (frame_time as i64 - (timestamp_nanos - next_frame) as i64).max(0) as u64,
                    );
                }
            }
        }
        return Duration::from_nanos(0);
    }
}

// ----------------------------------------------------------------------------

struct DisplayLinkWrapper {
    ptr: *mut display_link::DisplayLink,
}

unsafe impl Send for DisplayLinkWrapper {}
unsafe impl Sync for DisplayLinkWrapper {}

impl DisplayLinkWrapper {
    fn new(display_link: display_link::DisplayLink) -> Self {
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
