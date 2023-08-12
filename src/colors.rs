//! colors related functionality.

use super::*;

// Copied from rerun
#[repr(transparent)]
pub struct ColorRGBA(pub u32);

impl ColorRGBA {
    #[inline]
    pub fn to_array(self) -> [u8; 4] {
        [
            (self.0 >> 24) as u8,
            (self.0 >> 16) as u8,
            (self.0 >> 8) as u8,
            self.0 as u8,
        ]
    }
}

// Some colors I really liked from https://www.youtube.com/watch?v=kfM-yu0iQBk
pub const FREYA_ORANGE: ColorRGBA = ColorRGBA(0xeb790700);
pub const FREYA_GOLD: ColorRGBA = ColorRGBA(0xea9e3600);
pub const FREYA_RED: ColorRGBA = ColorRGBA(0xf8105300);
pub const FREYA_BLUE: ColorRGBA = ColorRGBA(0x30b5f700);
pub const FREYA_GREEN: ColorRGBA = ColorRGBA(0x0aeb9f00);
pub const FREYA_LIGHT_BLUE: ColorRGBA = ColorRGBA(0x72c5dd00);
pub const FREYA_GRAY: ColorRGBA = ColorRGBA(0xb2c5c500);
pub const FREYA_PINK: ColorRGBA = ColorRGBA(0xeaa48300);
pub const FREYA_LIGHT_GRAY: ColorRGBA = ColorRGBA(0xf4f5f800);
pub const FREYA_DARK_BLUE: ColorRGBA = ColorRGBA(0x4da7c200);
pub const FREYA_DARK_GREEN: ColorRGBA = ColorRGBA(0x37bda900);
pub const FREYA_DARK_RED: ColorRGBA = ColorRGBA(0xae204400);
pub const FREYA_VIOLET: ColorRGBA = ColorRGBA(0xa401ed00);
pub const FREYA_WHITE: ColorRGBA = ColorRGBA(0xfaf8fb00);
pub const FREYA_YELLOW: ColorRGBA = ColorRGBA(0xf7d45400);
pub const FREYA_LIGHT_YELLOW: ColorRGBA = ColorRGBA(0xead8ad00);
pub const FREYA_LIGHT_GREEN: ColorRGBA = ColorRGBA(0x6ec29c00);

// Returns the expected size of units depending on their type
pub fn get_unit_sized_color(unit_name: &str, user_id: i64) -> (f32, ColorRGBA) {
    let mut unit_size = 0.045;
    let color = match unit_name {
        "VespeneEDyser" => FREYA_LIGHT_GREEN,
        "SpacePlatformGeyser" => FREYA_LIGHT_GREEN,
        "LabMineralField" => {
            unit_size = 0.024;
            FREYA_LIGHT_BLUE
        }
        "LabMineralField750" => {
            unit_size = 0.036;
            FREYA_LIGHT_BLUE
        }
        "MineralField" => {
            unit_size = 0.048;
            FREYA_LIGHT_BLUE
        }
        "MineralField450" => {
            unit_size = 0.06;
            FREYA_LIGHT_BLUE
        }
        "MineralField750" => {
            unit_size = 0.072;
            FREYA_LIGHT_BLUE
        }
        "XelNagaTower" => {
            // This should be super transparent
            unit_size = 0.072;
            FREYA_WHITE
        }
        "RichMineralField" => FREYA_GOLD,
        "RichMineralField750" => FREYA_ORANGE,
        "DestructibleDebris6x6" => {
            unit_size = 0.18;
            FREYA_GRAY
        }
        "UnbuildablePlatesDestructible" => {
            unit_size = 0.06;
            FREYA_LIGHT_GRAY
        }
        "Overlord" => {
            unit_size = 0.06;
            FREYA_YELLOW
        }
        "SCV" | "Drone" | "Probe" | "Larva" => {
            unit_size = 0.03;
            FREYA_LIGHT_GRAY
        }
        "Hatchery" | "CommandCenter" | "Nexus" => {
            unit_size = 0.12;
            FREYA_PINK
        }
        "Broodling" => {
            unit_size = 0.006;
            FREYA_LIGHT_GRAY
        }
        _ => {
            // Ignore the Beacons for now.
            if !unit_name.starts_with("Beacon") {
                log!("Unknown unit name: '{}'", unit_name);
            }
            // Fallback to user color
            user_color(user_id)
        }
    };
    (unit_size, color)
}

pub fn user_color(user_id: i64) -> ColorRGBA {
    match user_id {
        0 => FREYA_LIGHT_GREEN,
        1 => FREYA_LIGHT_BLUE,
        2 => FREYA_LIGHT_GRAY,
        3 => FREYA_ORANGE,
        _ => FREYA_WHITE,
    }
}
