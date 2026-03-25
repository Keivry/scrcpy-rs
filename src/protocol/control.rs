// SPDX-License-Identifier: MIT OR Apache-2.0

use {
    byteorder::{BigEndian, WriteBytesExt},
    std::io::{Result, Write},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMessageType {
    InjectKeycode = 0,
    InjectText = 1,
    InjectTouchEvent = 2,
    InjectScrollEvent = 3,
    BackOrScreenOn = 4,
    ExpandNotificationPanel = 5,
    ExpandSettingsPanel = 6,
    CollapsePanels = 7,
    GetClipboard = 8,
    SetClipboard = 9,
    SetDisplayPower = 10,
    RotateDevice = 11,
    UhidCreate = 12,
    UhidInput = 13,
    UhidDestroy = 14,
    OpenHardKeyboardSettings = 15,
    StartApp = 16,
    ResetVideo = 17,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidKeyEventAction {
    Down = 0,
    Up = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidMotionEventAction {
    Down = 0,
    Up = 1,
    Move = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub screen_width: u16,
    pub screen_height: u16,
}

impl Position {
    pub fn new(x: u32, y: u32, screen_width: u16, screen_height: u16) -> Self {
        Self {
            x,
            y,
            screen_width,
            screen_height,
        }
    }

    fn serialize<W: Write>(&self, buf: &mut W) -> Result<()> {
        buf.write_u32::<BigEndian>(self.x)?;
        buf.write_u32::<BigEndian>(self.y)?;
        buf.write_u16::<BigEndian>(self.screen_width)?;
        buf.write_u16::<BigEndian>(self.screen_height)?;
        Ok(())
    }
}

pub const POINTER_ID_MOUSE: u64 = u64::MAX;
pub const POINTER_ID_GENERIC_FINGER: u64 = u64::MAX - 1;
pub const POINTER_ID_VIRTUAL_FINGER: u64 = u64::MAX - 2;

#[derive(Debug, Clone)]
pub enum ControlMessage {
    InjectKeycode {
        action: AndroidKeyEventAction,
        keycode: u32,
        repeat: u32,
        metastate: u32,
    },
    InjectText {
        text: String,
    },
    InjectTouchEvent {
        action: AndroidMotionEventAction,
        pointer_id: u64,
        position: Position,
        pressure: f32,
        action_button: u32,
        buttons: u32,
    },
    InjectScrollEvent {
        position: Position,
        hscroll: f32,
        vscroll: f32,
        buttons: u32,
    },
    BackOrScreenOn {
        action: AndroidKeyEventAction,
    },
    ExpandNotificationPanel,
    ExpandSettingsPanel,
    CollapsePanels,
    SetDisplayPower {
        on: bool,
    },
    RotateDevice,
}

impl ControlMessage {
    pub fn serialize(&self, buf: &mut Vec<u8>) -> Result<usize> {
        let start_len = buf.len();

        match self {
            Self::InjectKeycode {
                action,
                keycode,
                repeat,
                metastate,
            } => {
                buf.write_u8(ControlMessageType::InjectKeycode as u8)?;
                buf.write_u8(*action as u8)?;
                buf.write_u32::<BigEndian>(*keycode)?;
                buf.write_u32::<BigEndian>(*repeat)?;
                buf.write_u32::<BigEndian>(*metastate)?;
            }
            Self::InjectText { text } => {
                buf.write_u8(ControlMessageType::InjectText as u8)?;
                let text_bytes = text.as_bytes();
                // Truncate at a UTF-8 character boundary so we never split a multibyte
                // codepoint. The scrcpy server protocol limits injected text to 300 bytes.
                let byte_limit = text_bytes.len().min(300);
                let safe_limit = (0..=byte_limit)
                    .rev()
                    .find(|&i| text.is_char_boundary(i))
                    .unwrap_or(0);
                buf.write_u32::<BigEndian>(safe_limit as u32)?;
                buf.write_all(&text_bytes[..safe_limit])?;
            }
            Self::InjectTouchEvent {
                action,
                pointer_id,
                position,
                pressure,
                action_button,
                buttons,
            } => {
                buf.write_u8(ControlMessageType::InjectTouchEvent as u8)?;
                buf.write_u8(*action as u8)?;
                buf.write_u64::<BigEndian>(*pointer_id)?;
                position.serialize(buf)?;
                let pressure_fp = (pressure.clamp(0.0, 1.0) * 65535.0) as u16;
                buf.write_u16::<BigEndian>(pressure_fp)?;
                buf.write_u32::<BigEndian>(*action_button)?;
                buf.write_u32::<BigEndian>(*buttons)?;
            }
            Self::InjectScrollEvent {
                position,
                hscroll,
                vscroll,
                buttons,
            } => {
                buf.write_u8(ControlMessageType::InjectScrollEvent as u8)?;
                position.serialize(buf)?;
                let hscroll_norm = (hscroll / 16.0).clamp(-1.0, 1.0);
                let vscroll_norm = (vscroll / 16.0).clamp(-1.0, 1.0);
                buf.write_i16::<BigEndian>((hscroll_norm * 32767.0) as i16)?;
                buf.write_i16::<BigEndian>((vscroll_norm * 32767.0) as i16)?;
                buf.write_u32::<BigEndian>(*buttons)?;
            }
            Self::BackOrScreenOn { action } => {
                buf.write_u8(ControlMessageType::BackOrScreenOn as u8)?;
                buf.write_u8(*action as u8)?;
            }
            Self::ExpandNotificationPanel => {
                buf.write_u8(ControlMessageType::ExpandNotificationPanel as u8)?;
            }
            Self::ExpandSettingsPanel => {
                buf.write_u8(ControlMessageType::ExpandSettingsPanel as u8)?;
            }
            Self::CollapsePanels => {
                buf.write_u8(ControlMessageType::CollapsePanels as u8)?;
            }
            Self::SetDisplayPower { on } => {
                buf.write_u8(ControlMessageType::SetDisplayPower as u8)?;
                buf.write_u8(if *on { 1 } else { 0 })?;
            }
            Self::RotateDevice => {
                buf.write_u8(ControlMessageType::RotateDevice as u8)?;
            }
        }

        Ok(buf.len() - start_len)
    }

    pub fn touch_down(x: u32, y: u32, screen_width: u16, screen_height: u16) -> Self {
        Self::InjectTouchEvent {
            action: AndroidMotionEventAction::Down,
            pointer_id: POINTER_ID_MOUSE,
            position: Position::new(x, y, screen_width, screen_height),
            pressure: 1.0,
            action_button: 0,
            buttons: 0,
        }
    }

    pub fn touch_up(x: u32, y: u32, screen_width: u16, screen_height: u16) -> Self {
        Self::InjectTouchEvent {
            action: AndroidMotionEventAction::Up,
            pointer_id: POINTER_ID_MOUSE,
            position: Position::new(x, y, screen_width, screen_height),
            pressure: 1.0,
            action_button: 0,
            buttons: 0,
        }
    }

    pub fn touch_move(x: u32, y: u32, screen_width: u16, screen_height: u16) -> Self {
        Self::InjectTouchEvent {
            action: AndroidMotionEventAction::Move,
            pointer_id: POINTER_ID_MOUSE,
            position: Position::new(x, y, screen_width, screen_height),
            pressure: 1.0,
            action_button: 0,
            buttons: 0,
        }
    }

    pub fn key_down(keycode: u32, metastate: u32) -> Self {
        Self::InjectKeycode {
            action: AndroidKeyEventAction::Down,
            keycode,
            repeat: 0,
            metastate,
        }
    }

    pub fn key_up(keycode: u32, metastate: u32) -> Self {
        Self::InjectKeycode {
            action: AndroidKeyEventAction::Up,
            keycode,
            repeat: 0,
            metastate,
        }
    }

    pub fn scroll(
        x: u32,
        y: u32,
        screen_width: u16,
        screen_height: u16,
        hscroll: f32,
        vscroll: f32,
    ) -> Self {
        Self::InjectScrollEvent {
            position: Position::new(x, y, screen_width, screen_height),
            hscroll,
            vscroll,
            buttons: 0,
        }
    }
}
