use crate::domain::desktop::DesktopGeometry;
use std::time::Duration;

pub type Rect = [f32; 4];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Resources,
    Office,
    Tools,
}
impl Group {
    pub const ALL: [Self; 3] = [Self::Resources, Self::Office, Self::Tools];
    pub fn index(self) -> usize {
        match self {
            Self::Resources => 0,
            Self::Office => 1,
            Self::Tools => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Resources => "资源",
            Self::Office => "办公",
            Self::Tools => "工具",
        }
    }
    pub fn artwork(self, selected: bool) -> &'static str {
        match (self, selected) {
            (Self::Resources, false) => "resource-orb",
            (Self::Resources, true) => "resource-orb-selected",
            (Self::Office, false) => "office-orb",
            (Self::Office, true) => "office-orb-selected",
            (Self::Tools, false) => "tools-orb",
            (Self::Tools, true) => "tools-orb-selected",
        }
    }
}

pub struct Layout {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub anchor: [f32; 2],
    left: bool,
    flip: bool,
    scale: f32,
}
impl Layout {
    pub fn new(geometry: DesktopGeometry) -> Self {
        let [x0, y0, x1, y1] = geometry.work_area;
        let [left, top, right, bottom] = geometry.frame;
        let scale = ((x1 - x0) / 520.0).min((y1 - y0) / 460.0).clamp(0.01, 1.0);
        let size = [520.0 * scale, 460.0 * scale];
        let on_left = x1 - right < left - x0;
        let center_y = (top + bottom) * 0.5;
        let position = [
            (if on_left {
                left - size[0] - 6.0
            } else {
                right + 6.0
            })
            .clamp(x0, (x1 - size[0]).max(x0)),
            (center_y - size[1] * 0.5).clamp(y0, (y1 - size[1]).max(y0)),
        ];
        Self {
            position,
            size,
            anchor: [if on_left { size[0] } else { 0.0 }, center_y - position[1]],
            left: on_left,
            flip: center_y > (y0 + y1) * 0.5,
            scale,
        }
    }
    pub fn rect(&self, raw: Rect) -> Rect {
        let [mut x0, mut y0, mut x1, mut y1] = raw;
        if self.left {
            (x0, x1) = (520.0 - x1, 520.0 - x0);
        }
        if self.flip {
            (y0, y1) = (460.0 - y1, 460.0 - y0);
        }
        [
            x0 * self.scale,
            y0 * self.scale,
            x1 * self.scale,
            y1 * self.scale,
        ]
    }
    pub fn group(&self, group: Group, age: Duration, animated: bool) -> Rect {
        let [x, y] = [[95.0, 110.0], [158.0, 230.0], [104.0, 365.0]][group.index()];
        let final_rect = self.rect([x - 48.0, y - 48.0, x + 48.0, y + 62.0]);
        let t = motion(group.index(), age, animated);
        let end = [
            (final_rect[0] + final_rect[2]) * 0.5,
            (final_rect[1] + final_rect[3]) * 0.5,
        ];
        let dx = end[0] - self.anchor[0];
        let dy = end[1] - self.anchor[1];
        let angle = (-0.65 * (1.0 - t)).copysign(if self.left { -1.0 } else { 1.0 });
        let radius = 0.35 + 0.65 * t;
        let center = [
            self.anchor[0] + (dx * angle.cos() - dy * angle.sin()) * radius,
            self.anchor[1] + (dx * angle.sin() + dy * angle.cos()) * radius,
        ];
        let w = (final_rect[2] - final_rect[0]) * 0.5;
        let h = (final_rect[3] - final_rect[1]) * 0.5;
        [center[0] - w, center[1] - h, center[0] + w, center[1] + h]
    }
    pub fn contains(&self, screen: [f32; 2], rect: Rect) -> bool {
        contains(
            [screen[0] - self.position[0], screen[1] - self.position[1]],
            rect,
        )
    }
}
pub fn contains(point: [f32; 2], rect: Rect) -> bool {
    point[0] >= rect[0] && point[0] <= rect[2] && point[1] >= rect[1] && point[1] <= rect[3]
}
pub fn motion(index: usize, age: Duration, animated: bool) -> f32 {
    if !animated {
        return 1.0;
    }
    let t = ((age.as_secs_f32() - index as f32 * 0.035) / 0.180).clamp(0.0, 1.0);
    let u = t - 1.0;
    1.0 + 2.25 * u * u * u + 1.25 * u * u
}

#[derive(Default)]
pub struct MenuState {
    pub active: bool,
    pub group: Option<Group>,
    pub opened: Duration,
    pub selected_at: Duration,
    candidate: Option<(Group, Duration)>,
    leave: Option<Duration>,
    suppressed: bool,
}
impl MenuState {
    pub fn open(&mut self, now: Duration) {
        *self = Self {
            active: true,
            opened: now,
            selected_at: now,
            ..Default::default()
        };
    }
    pub fn close(&mut self) {
        self.active = false;
        self.group = None;
        self.candidate = None;
        self.suppressed = true;
    }
    pub fn update(&mut self, now: Duration, trigger: bool, inside: bool, pinned: bool) {
        if !trigger {
            self.suppressed = false;
        }
        if !self.active && trigger && !self.suppressed {
            self.open(now);
        }
        if !self.active {
            return;
        }
        if trigger || inside || pinned {
            self.leave = None;
        } else if let Some(since) = self.leave {
            if now.saturating_sub(since) >= Duration::from_millis(350) {
                self.close();
            }
        } else {
            self.leave = Some(now);
        }
    }
    pub fn hover(&mut self, now: Duration, group: Option<Group>, clicked: bool) {
        let Some(group) = group else {
            self.candidate = None;
            return;
        };
        if self.group == Some(group) {
            self.candidate = None;
            return;
        }
        if clicked
            || self.candidate.is_some_and(|(g, t)| {
                g == group && now.saturating_sub(t) >= Duration::from_millis(120)
            })
        {
            self.group = Some(group);
            self.selected_at = now;
            self.candidate = None;
        } else if self.candidate.map(|(g, _)| g) != Some(group) {
            self.candidate = Some((group, now));
        }
    }
    pub fn animating(&self, now: Duration, enabled: bool) -> bool {
        enabled
            && (now.saturating_sub(self.opened) < Duration::from_millis(250)
                || self.group.is_some()
                    && now.saturating_sub(self.selected_at) < Duration::from_millis(250))
    }
    pub fn next_input_check(&self, now: Duration) -> Option<Duration> {
        [
            self.leave.map(|t| t + Duration::from_millis(350)),
            self.candidate.map(|(_, t)| t + Duration::from_millis(120)),
        ]
        .into_iter()
        .flatten()
        .map(|deadline| deadline.saturating_sub(now).max(Duration::from_millis(1)))
        .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hover_intent_gaps_close_and_explicit_close_does_not_reopen_under_mouse() {
        let mut s = MenuState::default();
        let ms = Duration::from_millis;
        s.update(ms(0), true, false, false);
        s.hover(ms(0), Some(Group::Office), false);
        s.hover(ms(119), Some(Group::Office), false);
        assert_eq!(s.group, None);
        s.hover(ms(120), Some(Group::Office), false);
        assert_eq!(s.group, Some(Group::Office));
        s.update(ms(200), false, false, false);
        s.update(ms(500), false, true, false);
        assert!(s.active);
        s.update(ms(600), false, false, false);
        s.update(ms(950), false, false, false);
        assert!(!s.active);
        s.update(ms(1000), true, false, false);
        s.close();
        s.update(ms(1100), true, false, false);
        assert!(!s.active);
        s.update(ms(1200), false, false, false);
        s.update(ms(1300), true, false, false);
        assert!(s.active);
    }
    #[test]
    fn layout_mirrors_and_stays_in_work_area_including_negative_coordinates() {
        for area in [[0.0, 0.0, 1920.0, 1040.0], [-1920.0, -1080.0, 0.0, 0.0]] {
            for frame in [
                [area[0], area[1], area[0] + 72.0, area[1] + 84.0],
                [area[2] - 72.0, area[3] - 84.0, area[2], area[3]],
            ] {
                let l = Layout::new(DesktopGeometry {
                    frame,
                    work_area: area,
                });
                assert!(l.position[0] >= area[0] && l.position[1] >= area[1]);
                assert!(
                    l.position[0] + l.size[0] <= area[2] && l.position[1] + l.size[1] <= area[3]
                );
                for group in Group::ALL {
                    let r = l.group(group, Duration::from_secs(1), true);
                    assert!(r[0] >= 0.0 && r[1] >= 0.0 && r[2] <= l.size[0] && r[3] <= l.size[1]);
                }
            }
        }
        assert_eq!(motion(2, Duration::from_millis(250), true), 1.0);
        assert_eq!(motion(2, Duration::ZERO, false), 1.0);
    }
}
