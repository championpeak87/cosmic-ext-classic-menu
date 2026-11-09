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

#[cfg(test)]
mod tests {
    use super::SelectionStep::{Down, Left, Right, Up};
    use super::step_selection;

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
