use num_derive::{FromPrimitive, ToPrimitive};
use std::mem::size_of;

#[derive(Debug, FromPrimitive, ToPrimitive, Default)]
pub enum DisplayCommand {
    #[default]
    CmdClear = 0x0002,
    CmdCp437data = 0x0003,
    CmdCharBrightness = 0x0005,
    CmdBrightness = 0x0007,
    CmdHardReset = 0x000b,
    CmdFadeOut = 0x000d,
    CmdBitmapLegacy = 0x0010,
    CmdBitmapLinear = 0x0012,
    CmdBitmapLinearWin = 0x0013,
    CmdBitmapLinearAnd = 0x0014,
    CmdBitmapLinearOr = 0x0015,
    CmdBitmapLinearXor = 0x0016,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct HdrWindow {
    pub command: DisplayCommand,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/*
#[repr(C)]
pub struct HdrBitmap {
    pub command: DisplayCommand,
    pub offset: u16,
    pub length: u16,
    pub subcommand: DisplaySubcommand,
    reserved: u16,
}
*/

#[repr(u16)]
#[derive(Debug, FromPrimitive, ToPrimitive)]
pub enum DisplaySubcommand {
    SubCmdBitmapNormal = 0x0,
    SubCmdBitmapCompressZ = 0x677a,
    SubCmdBitmapCompressBz = 0x627a,
    SubCmdBitmapCompressLz = 0x6c7a,
    SubCmdBitmapCompressZs = 0x7a73,
}

pub const TILE_SIZE: u16 = 8;
pub const TILE_WIDTH: u16 = 56;
pub const TILE_HEIGHT: u16 = 20;
pub const PIXEL_WIDTH: u16 = TILE_WIDTH * TILE_SIZE;
pub const PIXEL_HEIGHT: u16 = TILE_HEIGHT * TILE_SIZE;
pub const PIXEL_COUNT: usize = PIXEL_WIDTH as usize * PIXEL_HEIGHT as usize;

#[derive(Debug)]
pub enum ReadHeaderError {
    BufferTooSmall,
    WrongCommandEndianness(u16, DisplayCommand),
    InvalidCommand(u16),
}

pub fn read_header(buffer: &[u8]) -> Result<HdrWindow, ReadHeaderError> {
    if buffer.len() < size_of::<HdrWindow>() {
        return Err(ReadHeaderError::BufferTooSmall);
    }

    let command_u16 = read_beu16(&buffer[0..=1]);
    return match num::FromPrimitive::from_u16(command_u16) {
        Some(command) => Ok(HdrWindow {
            command,
            x: read_beu16(&buffer[2..=3]),
            y: read_beu16(&buffer[4..=5]),
            w: read_beu16(&buffer[6..=7]),
            h: read_beu16(&buffer[8..=9]),
        }),
        None => {
            let maybe_command: Option<DisplayCommand> =
                num::FromPrimitive::from_u16(u16::swap_bytes(command_u16));
            return match maybe_command {
                None => Err(ReadHeaderError::InvalidCommand(command_u16)),
                Some(command) => Err(ReadHeaderError::WrongCommandEndianness(
                    command_u16,
                    command,
                )),
            };
        }
    };
}

fn read_beu16(buffer: &[u8]) -> u16 {
    let buffer: [u8; 2] = buffer
        .try_into()
        .expect("cannot read u16 from buffer with size != 2");
    return u16::from_be_bytes(buffer);
}
