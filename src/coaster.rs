//! Rollercoaster logic.

use std::collections::VecDeque;
use std::fmt::Write;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Movement {
    UpLeft,
    Up,
    UpRight,
    Left,
    Right,
    DownLeft,
    Down,
    DownRight,
}
impl Movement {
    /// Converts this movement to coordinates.
    ///
    /// Assumes that X is positive-right and Y is positive-down (UI coordinates, not standard
    /// geometrical coordinates).
    ///
    /// Returned in order (Y, X) to match ANSI escapes.
    pub fn to_coordinates(&self) -> (isize, isize) {
        match self {
            Self::UpLeft => (-1, -1),
            Self::Up => (-1, 0),
            Self::UpRight => (-1, 1),
            Self::Left => (0, -1),
            Self::Right => (0, 1),
            Self::DownLeft => (1, -1),
            Self::Down => (1, 0),
            Self::DownRight => (1, 1),
        }
    }
}


/// Decodes rollercoaster movements from a string representation.
///
/// The string representation mirrors the layout of a computer's numeric keypad:
///
/// * 7 = up-left
/// * 8 = up
/// * 9 = up-right
/// * 4 = left
/// * 6 = right
/// * 1 = down-left
/// * 2 = down
/// * 3 = down-right
///
/// If any other character is encountered, the function returns `None`.
pub(crate) fn decode_movements(movements: &str) -> Option<Vec<Movement>> {
    let mut ret = Vec::with_capacity(movements.len());
    for mov in movements.chars() {
        ret.push(
            match mov {
                '7' => Movement::UpLeft,
                '8' => Movement::Up,
                '9' => Movement::UpRight,
                '4' => Movement::Left,
                '6' => Movement::Right,
                '1' => Movement::DownLeft,
                '2' => Movement::Down,
                '3' => Movement::DownRight,
                _ => return None,
            }
        );
    }
    Some(ret)
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Rollercoaster {
    base_lines: Vec<String>,
    train: String,
    train_start: Vec<(isize, isize)>,
    movements: Vec<Movement>,

    train_positions: VecDeque<(isize, isize)>,
    frame_index: usize,
}
impl Rollercoaster {
    pub fn new<
        L: Into<Vec<String>>,
        T: Into<String>,
        S: Into<Vec<(isize, isize)>>,
        M: Into<Vec<Movement>>,
    >(
        base_lines: L,
        train: T,
        train_start: S,
        movements: M,
    ) -> Self {
        let train_start_vec = train_start.into();
        let ret = Self {
            base_lines: base_lines.into(),
            train: train.into(),
            train_start: train_start_vec.clone(),
            movements: movements.into(),

            train_positions: VecDeque::from(train_start_vec),
            frame_index: 0,
        };
        assert_ne!(ret.base_lines.len(), 0);
        assert_ne!(ret.base_lines[0].len(), 0);
        assert_ne!(ret.train.len(), 0);
        assert_eq!(ret.train.len(), ret.train_start.len());
        ret
    }

    pub fn get_total_frames(&self) -> usize {
        self.movements.len()
    }

    pub fn get_base_frame(&self) -> String {
        let mut ret = String::new();
        for line in &self.base_lines {
            ret.push_str(line);
            ret.push_str("\r\n");
        }
        ret
    }

    pub fn get_width(&self) -> isize {
        self.base_lines.iter()
            .map(|bl| bl.chars().count())
            .max().unwrap() as isize
    }

    pub fn get_height(&self) -> isize {
        self.base_lines.len() as isize
    }

    pub fn reset(&mut self) {
        self.frame_index = 0;

        self.train_positions.clear();
        for &pos in &self.train_start {
            self.train_positions.push_back(pos);
        }
    }

    pub fn advance(&mut self) -> Option<String> {
        let mut ret = String::new();

        if self.frame_index >= self.movements.len() {
            return None;
        }

        // return the last character of the train to its original state
        let &(last_seg_row, last_seg_col) = self.train_positions.back().unwrap();
        if last_seg_row >= 0 && last_seg_col >= 0 && last_seg_row < self.get_height() && last_seg_col < self.get_width() {
            let base_char = self.base_lines[last_seg_row as usize]
                .chars()
                .nth(last_seg_col as usize)
                .unwrap_or(' ');
            write!(ret, "\x1B[{};{}H{}", last_seg_row+1, last_seg_col+1, base_char).unwrap();
        }

        // drop the last train position
        self.train_positions.pop_back();

        // calculate the new position by looking at the movement
        let (cur_row, cur_col) = *self.train_positions.front().unwrap();
        let (move_row, move_col) = self.movements[self.frame_index].to_coordinates();
        self.train_positions.push_front((cur_row + move_row, cur_col + move_col));

        // update the train positions
        let mut last_pos = None;
        for (&(pos_row, pos_col), train_char) in self.train_positions.iter().zip(self.train.chars()) {
            if pos_row < 0 || pos_col < 0 {
                continue;
            }
            if pos_row >= self.get_height() || pos_col >= self.get_width() {
                continue;
            }

            let mut set_new_pos = true;
            if let Some((last_row, last_col)) = last_pos {
                if pos_row == last_row && pos_col == last_col + 1 {
                    // it's the next character in the line; we need not reposition the cursor
                    set_new_pos = false;
                }
            }

            if set_new_pos {
                write!(ret, "\x1B[{};{}H", pos_row+1, pos_col+1).unwrap();
            }
            write!(ret, "{}", train_char).unwrap();

            last_pos = Some((pos_row, pos_col));
        }

        // increase the frame index
        self.frame_index += 1;

        Some(ret)
    }
}
