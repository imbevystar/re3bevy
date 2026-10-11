//! re3 reference: references/re-GTA/src/core/FrontEndControls.cpp
//! re3 reference: references/re-GTA/src/core/FrontEndControls.h
//!
//! Frontend control widgets as state machines. Drawing is emitted as intents
//! and resolved by the later renderer/frontend draw pass.

use re3bevy_core::core::Coordinates::CVector2D;

use crate::{objects::ParticleObject::CRGBA, text::Text::GtaWideString};

pub const NUM_MULTICHOICE_OPTIONS: usize = 16;
pub const NUM_LINELISTER_LINES: usize = 50;
pub const NUM_LINELISTER_LINES_TOTAL: usize = NUM_LINELISTER_LINES + 15;
pub const NUM_PAGE_WIDGETS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CTriggerCaller {
    pub has_trigger: bool,
}

impl CTriggerCaller {
    pub fn set_trigger(&mut self) {
        if !self.has_trigger {
            self.has_trigger = true;
        }
    }

    pub fn call_trigger(&mut self) -> bool {
        let had_trigger = self.has_trigger;
        self.has_trigger = false;
        had_trigger
    }

    pub fn can_call(&self) -> bool {
        self.has_trigger
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CPlaceableText {
    pub position: CVector2D,
    pub color: CRGBA,
    pub text: Option<GtaWideString>,
}

impl Default for CPlaceableText {
    fn default() -> Self {
        Self {
            position: CVector2D::new(0.0, 0.0),
            color: CRGBA::new(255, 255, 255, 255),
            text: None,
        }
    }
}

impl CPlaceableText {
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = CVector2D::new(x, y);
    }

    pub fn set_color(&mut self, color: CRGBA) {
        self.color = color;
    }

    pub fn set_alpha(&mut self, alpha: u8) {
        self.color.alpha = alpha;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CShadowInfo {
    pub right_justify: bool,
    pub drop_shadow: bool,
    pub shadow_color: CRGBA,
    pub shadow_offset: CVector2D,
}

impl Default for CShadowInfo {
    fn default() -> Self {
        Self {
            right_justify: false,
            drop_shadow: false,
            shadow_color: CRGBA::new(255, 255, 255, 255),
            shadow_offset: CVector2D::new(-1.0, -1.0),
        }
    }
}

impl CShadowInfo {
    pub fn set_shadows(&mut self, drop_shadows: bool, color: CRGBA, offset: CVector2D) {
        self.drop_shadow = drop_shadows;
        self.shadow_color = color;
        self.shadow_offset = offset;
    }

    pub fn set_alpha(&mut self, alpha: u8) {
        self.shadow_color.alpha = alpha;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CSelectable {
    pub selected: bool,
    pub selected_color: CRGBA,
}

impl Default for CSelectable {
    fn default() -> Self {
        Self {
            selected: false,
            selected_color: CRGBA::new(255, 255, 255, 255),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CPlaceableShOption {
    pub text: CPlaceableText,
    pub shadow: CShadowInfo,
    pub selectable: CSelectable,
}

impl CPlaceableShOption {
    pub fn set_colors(&mut self, normal: CRGBA, selection: CRGBA) {
        self.text.set_color(normal);
        self.selectable.selected_color = selection;
    }

    pub fn set_alpha(&mut self, alpha: u8) {
        self.text.set_alpha(alpha);
        self.shadow.set_alpha(alpha);
        self.selectable.selected_color.alpha = alpha;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMenuMultiChoice {
    pub num_options: usize,
    pub title: CPlaceableText,
    pub options: [CPlaceableShOption; NUM_MULTICHOICE_OPTIONS],
    pub cursor: i32,
}

impl Default for CMenuMultiChoice {
    fn default() -> Self {
        Self {
            num_options: 0,
            title: CPlaceableText::default(),
            options: std::array::from_fn(|_| CPlaceableShOption::default()),
            cursor: -1,
        }
    }
}

impl CMenuMultiChoice {
    pub fn add_title(&mut self, text: GtaWideString, x: f32, y: f32) {
        self.title.text = Some(text);
        self.title.set_position(x, y);
    }

    pub fn add_option(
        &mut self,
        text: GtaWideString,
        x: f32,
        y: f32,
        selected: bool,
    ) -> Option<usize> {
        if self.num_options >= NUM_MULTICHOICE_OPTIONS {
            return None;
        }
        let index = self.num_options;
        self.options[index].text.text = Some(text);
        self.options[index].text.set_position(x, y);
        self.options[index].selectable.selected = selected;
        self.num_options += 1;
        Some(index)
    }

    pub fn go_next(&mut self) -> bool {
        if self.num_options == 0 {
            self.deactivate_menu();
            return false;
        }
        self.cursor += 1;
        if self.cursor >= self.num_options as i32 {
            self.cursor = 0;
        }
        true
    }

    pub fn go_prev(&mut self) -> bool {
        if self.num_options == 0 {
            self.deactivate_menu();
            return false;
        }
        self.cursor -= 1;
        if self.cursor < 0 {
            self.cursor = self.num_options as i32 - 1;
        }
        true
    }

    pub fn go_first(&mut self) -> bool {
        if self.num_options == 0 {
            return false;
        }
        self.cursor = 0;
        true
    }

    pub fn go_last(&mut self) -> bool {
        if self.num_options == 0 {
            return false;
        }
        self.cursor = self.num_options as i32 - 1;
        true
    }

    pub fn activate_menu(&mut self, first: bool) {
        if first {
            self.go_first();
        } else {
            self.go_last();
        }
    }

    pub fn deactivate_menu(&mut self) {
        self.cursor = -1;
    }

    pub fn get_menu_selection(&self) -> i32 {
        self.cursor
    }

    pub fn set_menu_selection(&mut self, selection: i32) {
        if selection >= 0 && selection < self.num_options as i32 {
            self.cursor = selection;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMenuOnOff {
    pub title: CPlaceableShOption,
    pub options: [CPlaceableText; 2],
    pub active: bool,
    pub option_type: i32,
}

impl Default for CMenuOnOff {
    fn default() -> Self {
        Self {
            title: CPlaceableShOption::default(),
            options: [CPlaceableText::default(), CPlaceableText::default()],
            active: false,
            option_type: 0,
        }
    }
}

impl CMenuOnOff {
    pub fn select_current_option_under_cursor(&mut self) {
        self.title.selectable.selected = !self.title.selectable.selected;
    }

    pub fn activate_menu(&mut self) {
        self.active = true;
    }

    pub fn deactivate_menu(&mut self) {
        self.active = false;
    }

    pub fn get_menu_selection(&self) -> i32 {
        i32::from(self.title.selectable.selected)
    }

    pub fn set_menu_selection(&mut self, selection: i32) {
        self.title.selectable.selected = selection != 0;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMenuSlider {
    pub value: i32,
    pub active: bool,
    pub change_step: i32,
}

impl Default for CMenuSlider {
    fn default() -> Self {
        Self {
            value: 0,
            active: false,
            change_step: 50,
        }
    }
}

impl CMenuSlider {
    pub fn go_left(&mut self) -> bool {
        self.value = (self.value - self.change_step).max(0);
        true
    }

    pub fn go_right(&mut self) -> bool {
        self.value = (self.value + self.change_step).min(1000);
        true
    }

    pub fn activate_menu(&mut self) {
        self.active = true;
    }

    pub fn deactivate_menu(&mut self) {
        self.active = false;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMenuLineLister {
    pub num_lines: usize,
    pub width: f32,
    pub height: f32,
    pub scroll_position: f32,
    pub scroll_speed: f32,
    pub line_spacing: f32,
    pub lines_left: [CPlaceableText; NUM_LINELISTER_LINES_TOTAL],
    pub lines_right: [CPlaceableText; NUM_LINELISTER_LINES_TOTAL],
    pub line_alphas: [i32; NUM_LINELISTER_LINES_TOTAL],
    pub line_fade: [i32; NUM_LINELISTER_LINES_TOTAL],
}

impl Default for CMenuLineLister {
    fn default() -> Self {
        Self {
            num_lines: 0,
            width: 0.0,
            height: 0.0,
            scroll_position: 0.0,
            scroll_speed: 1.0,
            line_spacing: 15.0,
            lines_left: std::array::from_fn(|_| CPlaceableText::default()),
            lines_right: std::array::from_fn(|_| CPlaceableText::default()),
            line_alphas: [0; NUM_LINELISTER_LINES_TOTAL],
            line_fade: [0; NUM_LINELISTER_LINES_TOTAL],
        }
    }
}

impl CMenuLineLister {
    pub fn reset_number_of_text_lines(&mut self) {
        self.num_lines = 0;
        self.line_alphas = [0; NUM_LINELISTER_LINES_TOTAL];
        self.line_fade = [0; NUM_LINELISTER_LINES_TOTAL];
        for i in 0..NUM_LINELISTER_LINES_TOTAL {
            self.lines_left[i].text = None;
            self.lines_right[i].text = None;
        }
    }

    pub fn add_text_line(&mut self, left: GtaWideString, right: GtaWideString) -> bool {
        if self.num_lines == NUM_LINELISTER_LINES {
            return false;
        }
        let index = self.num_lines;
        self.lines_left[index].text = Some(left);
        self.lines_left[index].set_position(0.0, self.line_spacing * (index + 15) as f32);
        self.lines_right[index].text = Some(right);
        self.lines_right[index].position = self.lines_left[index].position;
        self.num_lines += 1;
        true
    }

    pub fn tick_scroll(&mut self) {
        let n = self.num_lines + 15;
        self.scroll_position += self.scroll_speed;
        if self.scroll_speed > 0.0 {
            if self.scroll_position > n as f32 * self.line_spacing {
                self.scroll_position = 0.0;
            }
        } else if self.scroll_position < 0.0 {
            self.scroll_position = n as f32 * self.line_spacing;
        }
        self.scroll_speed = 1.0;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMenuPage {
    pub num_controls: usize,
    pub cursor: usize,
    pub controls_can_activate: [bool; NUM_PAGE_WIDGETS],
}

impl Default for CMenuPage {
    fn default() -> Self {
        Self {
            num_controls: 0,
            cursor: 0,
            controls_can_activate: [false; NUM_PAGE_WIDGETS],
        }
    }
}

impl CMenuPage {
    pub fn initialise(&mut self) {
        *self = Self::default();
    }

    pub fn add_menu(&mut self, can_activate: bool) -> bool {
        if self.num_controls >= NUM_PAGE_WIDGETS {
            return false;
        }
        self.controls_can_activate[self.num_controls] = can_activate;
        self.num_controls += 1;
        true
    }

    pub fn activate_page(&mut self) {
        self.cursor = 0;
        if self.num_controls == 0 {
            return;
        }
        while !self.controls_can_activate[self.cursor] {
            self.cursor = (self.cursor + 1) % self.num_controls;
        }
    }

    pub fn go_up_menu_on_page(&mut self) {
        if self.num_controls == 0 {
            return;
        }
        loop {
            self.cursor = if self.cursor == 0 {
                self.num_controls - 1
            } else {
                self.cursor - 1
            };
            if self.controls_can_activate[self.cursor] {
                break;
            }
        }
    }

    pub fn go_down_menu_on_page(&mut self) {
        if self.num_controls == 0 {
            return;
        }
        loop {
            self.cursor = (self.cursor + 1) % self.num_controls;
            if self.controls_can_activate[self.cursor] {
                break;
            }
        }
    }
}
