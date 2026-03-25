// SPDX-License-Identifier: MIT OR Apache-2.0

use {
    crate::{Result, protocol::control::ControlMessage},
    parking_lot::Mutex,
    std::{io::Write, net::TcpStream, sync::Arc},
    tracing::trace,
};

/// Thread-safe handle for sending scrcpy control messages over the control channel.
///
/// Cloning a `ControlSender` is cheap — all clones share the same underlying TCP
/// stream (guarded by a `Mutex`) and the same screen-size state.
#[derive(Clone)]
pub struct ControlSender {
    stream: Arc<Mutex<TcpStream>>,
    screen_size: Arc<Mutex<(u16, u16)>>,
}

impl ControlSender {
    pub fn new(stream: TcpStream, screen_width: u16, screen_height: u16) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            screen_size: Arc::new(Mutex::new((screen_width, screen_height))),
        }
    }

    pub fn update_screen_size(&self, width: u16, height: u16) {
        *self.screen_size.lock() = (width, height);
        trace!("Updated control sender screen size to {}x{}", width, height);
    }

    pub fn get_screen_size(&self) -> (u16, u16) { *self.screen_size.lock() }

    fn send_message(&self, msg: &ControlMessage) -> Result<()> {
        let mut buf = Vec::with_capacity(64);
        msg.serialize(&mut buf)?;

        let mut stream = self.stream.lock();
        stream.write_all(&buf)?;
        trace!("Sent control message: {:?}", msg);
        Ok(())
    }

    pub fn send_touch_down(&self, x: u32, y: u32) -> Result<()> {
        let (width, height) = self.get_screen_size();
        self.send_message(&ControlMessage::touch_down(x, y, width, height))
    }

    pub fn send_touch_up(&self, x: u32, y: u32) -> Result<()> {
        let (width, height) = self.get_screen_size();
        self.send_message(&ControlMessage::touch_up(x, y, width, height))
    }

    pub fn send_touch_move(&self, x: u32, y: u32) -> Result<()> {
        let (width, height) = self.get_screen_size();
        self.send_message(&ControlMessage::touch_move(x, y, width, height))
    }

    pub fn send_key_down(&self, keycode: u32, metastate: u32) -> Result<()> {
        self.send_message(&ControlMessage::key_down(keycode, metastate))
    }

    pub fn send_key_up(&self, keycode: u32, metastate: u32) -> Result<()> {
        self.send_message(&ControlMessage::key_up(keycode, metastate))
    }

    pub fn send_key_press(&self, keycode: u32, metastate: u32) -> Result<()> {
        self.send_key_down(keycode, metastate)?;
        self.send_key_up(keycode, metastate)
    }

    pub fn send_set_display_power(&self, on: bool) -> Result<()> {
        self.send_message(&ControlMessage::SetDisplayPower { on })
    }

    pub fn send_screen_off(&self) -> Result<()> { self.send_set_display_power(false) }

    pub fn send_text(&self, text: &str) -> Result<()> {
        self.send_message(&ControlMessage::InjectText {
            text: text.to_string(),
        })
    }

    pub fn send_scroll(&self, x: u32, y: u32, hscroll: f32, vscroll: f32) -> Result<()> {
        let (width, height) = self.get_screen_size();
        self.send_message(&ControlMessage::scroll(
            x, y, width, height, hscroll, vscroll,
        ))
    }

    pub fn send_custom(&self, msg: &ControlMessage) -> Result<()> { self.send_message(msg) }
}
