use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PetKind {
    #[default]
    Cat,
    Fox,
    Owl,
}
impl PetKind {
    pub const ALL: [Self; 3] = [Self::Cat, Self::Fox, Self::Owl];
    pub fn index(self) -> usize {
        match self {
            Self::Cat => 0,
            Self::Fox => 1,
            Self::Owl => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Cat => "小猫",
            Self::Fox => "狐狸",
            Self::Owl => "猫头鹰",
        }
    }
}
#[derive(Clone, Copy)]
pub struct DesktopGeometry {
    pub frame: [f32; 4],
    pub work_area: [f32; 4],
}
pub fn flyout_position(geometry: DesktopGeometry, size: [f32; 2]) -> [f32; 2] {
    let [left, top, right, bottom] = geometry.frame;
    let [min_x, min_y, max_x, max_y] = geometry.work_area;
    let x = if right + size[0] <= max_x {
        right
    } else {
        left - size[0]
    };
    let y = if top + size[1] <= max_y {
        top
    } else {
        bottom - size[1]
    };
    [
        x.clamp(min_x, (max_x - size[0]).max(min_x)),
        y.clamp(min_y, (max_y - size[1]).max(min_y)),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flyout_uses_adjacent_side_and_stays_in_negative_coordinate_work_area() {
        let area = [-1920.0, 0.0, 0.0, 1080.0];
        assert_eq!(
            flyout_position(
                DesktopGeometry {
                    frame: [-72.0, 1000.0, 0.0, 1072.0],
                    work_area: area
                },
                [300.0, 350.0]
            ),
            [-372.0, 722.0]
        );
        assert_eq!(
            flyout_position(
                DesktopGeometry {
                    frame: [-1920.0, 0.0, -1848.0, 72.0],
                    work_area: area
                },
                [300.0, 350.0]
            ),
            [-1848.0, 0.0]
        );
    }
}
