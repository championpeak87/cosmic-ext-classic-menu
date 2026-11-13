//! Moving the highlighted app with the arrow keys.

/// A direction to move the highlighted app in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionStep {
    Up,
    Down,
    Left,
    Right,
}

/// The app highlighted after `step`, of `len` apps laid out in rows of
/// `columns`.
///
/// Left and right follow the order of the apps, so they continue on the
/// neighboring row at the end of one. They do nothing in a single column.
pub fn step_selection(
    current: Option<usize>,
    step: SelectionStep,
    len: usize,
    columns: usize,
) -> Option<usize> {
    let horizontal = matches!(step, SelectionStep::Left | SelectionStep::Right);
    if len == 0 || (horizontal && columns <= 1) {
        return current;
    }

    let last = len - 1;
    match (current, step) {
        (None, SelectionStep::Down | SelectionStep::Right) => Some(0),
        (None, SelectionStep::Up | SelectionStep::Left) => None,
        (Some(index), SelectionStep::Up) if index >= columns => Some(index - columns),
        (Some(index), SelectionStep::Up) => Some(index),
        (Some(index), SelectionStep::Down) => Some((index + columns).min(last)),
        (Some(index), SelectionStep::Left) => Some(index.saturating_sub(1)),
        (Some(index), SelectionStep::Right) => Some((index + 1).min(last)),
    }
}

/// How the apps of the app view are laid out in rows: the apps leading
/// all apps, if any, then all apps from a row of their own, with a
/// separator of `gap` between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppRows {
    pub len: usize,
    pub columns: usize,
    /// How many apps lead all apps. Without any app after them, there is
    /// nothing to separate them from, so they count as none.
    pub leading: usize,
    pub row_height: f32,
    pub gap: f32,
}

impl AppRows {
    fn columns(&self) -> usize {
        self.columns.max(1)
    }

    /// How many apps are before the separator, if there is one.
    fn leading(&self) -> Option<usize> {
        (self.leading > 0 && self.leading < self.len).then_some(self.leading)
    }

    /// The row all apps start on, below the separator, if there is one.
    pub fn break_row(&self) -> Option<usize> {
        self.leading().map(|leading| leading.div_ceil(self.columns()))
    }

    pub fn rows(&self) -> usize {
        match self.leading() {
            Some(leading) => {
                leading.div_ceil(self.columns()) + (self.len - leading).div_ceil(self.columns())
            }
            None => self.len.div_ceil(self.columns()),
        }
    }

    /// Indices of the apps in `row`.
    pub fn row_range(&self, row: usize) -> std::ops::Range<usize> {
        let columns = self.columns();
        let (first, end) = match (self.leading(), self.break_row()) {
            (Some(leading), Some(break_row)) if row < break_row => {
                (row * columns, leading)
            }
            (Some(leading), Some(break_row)) => {
                (leading + (row - break_row) * columns, self.len)
            }
            _ => (row * columns, self.len),
        };
        first.min(end)..(first + columns).min(end)
    }

    pub fn row_of(&self, index: usize) -> usize {
        let columns = self.columns();
        match (self.leading(), self.break_row()) {
            (Some(leading), Some(break_row)) if index >= leading => {
                break_row + (index - leading) / columns
            }
            _ => index / columns,
        }
    }

    /// Top of `row`, below the separator if it is past it.
    pub fn top(&self, row: usize) -> f32 {
        let gap = if self.break_row().is_some_and(|break_row| row >= break_row) {
            self.gap
        } else {
            0.
        };
        row as f32 * self.row_height + gap
    }

    pub fn height(&self) -> f32 {
        self.top(self.rows())
    }

    /// The row at `y`, the separator counting as the row below it.
    pub fn row_at(&self, y: f32) -> usize {
        if self.row_height <= 0. {
            return 0;
        }
        let row = (y.max(0.) / self.row_height).floor() as usize;
        match self.break_row() {
            Some(break_row) if row >= break_row => {
                let below = ((y - self.top(break_row)).max(0.) / self.row_height).floor() as usize;
                break_row + below
            }
            _ => row,
        }
    }

    /// The app highlighted after `step`, as [`step_selection`] does, up and
    /// down keeping to the column across the separator.
    pub fn step(&self, current: Option<usize>, step: SelectionStep) -> Option<usize> {
        let (Some(index), SelectionStep::Up | SelectionStep::Down, Some(_)) =
            (current, step, self.leading())
        else {
            return step_selection(current, step, self.len, self.columns);
        };
        let row = self.row_of(index);
        let column = index - self.row_range(row).start;
        let target = match step {
            SelectionStep::Up if row == 0 => return Some(index),
            SelectionStep::Up => row - 1,
            _ if row + 1 >= self.rows() => return Some(self.len - 1),
            _ => row + 1,
        };
        let range = self.row_range(target);
        Some((range.start + column).min(range.end - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::SelectionStep::{Down, Left, Right, Up};
    use super::{AppRows, step_selection};

    /// 3 leading apps, then 9 apps, in rows of 5.
    fn rows() -> AppRows {
        AppRows {
            len: 12,
            columns: 5,
            leading: 3,
            row_height: 10.,
            gap: 4.,
        }
    }

    #[test]
    fn all_apps_start_on_a_row_of_their_own() {
        let rows = rows();
        assert_eq!(rows.rows(), 3);
        assert_eq!(rows.row_range(0), 0..3);
        assert_eq!(rows.row_range(1), 3..8);
        assert_eq!(rows.row_range(2), 8..12);
        assert_eq!(rows.row_of(2), 0);
        assert_eq!(rows.row_of(3), 1);
        assert_eq!(rows.row_of(11), 2);
    }

    #[test]
    fn the_separator_pushes_the_rows_below_it_down() {
        let rows = rows();
        assert_eq!(rows.top(0), 0.);
        assert_eq!(rows.top(1), 14.);
        assert_eq!(rows.height(), 34.);
        assert_eq!(rows.row_at(9.), 0);
        assert_eq!(rows.row_at(12.), 1);
        assert_eq!(rows.row_at(24.), 2);
    }

    #[test]
    fn without_apps_after_the_leading_ones_there_is_no_separator() {
        let rows = AppRows { len: 3, ..rows() };
        assert_eq!(rows.break_row(), None);
        assert_eq!(rows.height(), 10.);
        let rows = AppRows { leading: 0, ..rows };
        assert_eq!(rows.break_row(), None);
    }

    #[test]
    fn vertical_moves_keep_to_the_column_across_the_separator() {
        let rows = rows();
        assert_eq!(rows.step(Some(1), Down), Some(4));
        assert_eq!(rows.step(Some(4), Up), Some(1));
        // Past the end of the shorter row above.
        assert_eq!(rows.step(Some(7), Up), Some(2));
        assert_eq!(rows.step(Some(7), Down), Some(11));
        assert_eq!(rows.step(Some(9), Down), Some(11));
        assert_eq!(rows.step(Some(2), Right), Some(3));
    }

    #[test]
    fn right_overflows_to_the_next_row() {
        assert_eq!(step_selection(Some(4), Right, 12, 5), Some(5));
    }

    #[test]
    fn left_overflows_to_the_previous_row() {
        assert_eq!(step_selection(Some(5), Left, 12, 5), Some(4));
    }

    #[test]
    fn horizontal_moves_stop_at_the_ends() {
        assert_eq!(step_selection(Some(0), Left, 12, 5), Some(0));
        assert_eq!(step_selection(Some(11), Right, 12, 5), Some(11));
    }

    #[test]
    fn horizontal_moves_do_nothing_in_a_single_column() {
        assert_eq!(step_selection(Some(3), Right, 12, 1), Some(3));
        assert_eq!(step_selection(Some(3), Left, 12, 1), Some(3));
        assert_eq!(step_selection(None, Right, 12, 1), None);
    }

    #[test]
    fn vertical_moves_go_a_row_at_a_time() {
        assert_eq!(step_selection(Some(7), Up, 12, 5), Some(2));
        assert_eq!(step_selection(Some(2), Up, 12, 5), Some(2));
        assert_eq!(step_selection(Some(2), Down, 12, 5), Some(7));
        assert_eq!(step_selection(Some(9), Down, 12, 5), Some(11));
    }

    #[test]
    fn starts_from_the_first_app_only_going_forward() {
        assert_eq!(step_selection(None, Down, 12, 5), Some(0));
        assert_eq!(step_selection(None, Right, 12, 5), Some(0));
        assert_eq!(step_selection(None, Up, 12, 5), None);
        assert_eq!(step_selection(None, Left, 12, 5), None);
    }

    #[test]
    fn nothing_to_select_without_apps() {
        assert_eq!(step_selection(None, Down, 0, 5), None);
    }
}
