//! Gomoku game
//!
//! Check struct [`Gomoku`](https://docs.rs/gamie/*/gamie/gomoku/struct.Gomoku.html) for more information
//!
//! # Examples
//!
//! ```rust
//! # fn gomoku() {
//! use gamie::gomoku::{Gomoku, Player as GomokuPlayer};
//!
//! let mut game = Gomoku::new().unwrap();
//! game.place(GomokuPlayer::Player0, 7, 8).unwrap();
//! game.place(GomokuPlayer::Player1, 8, 7).unwrap();
//! // ...
//! # }
//! ```

use crate::std_lib::{iter, Box, Infallible};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use snafu::Snafu;

/// Gomoku
///
/// Passing an invalid position to a method will cause panic. Check the target position validity first when dealing with user input
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Gomoku {
    board: [[Option<Player>; 15]; 15],
    next: Player,
    status: GameState,
}

/// Players
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Player {
    Player0,
    Player1,
}

impl Player {
    /// Get the opposite player
    pub fn other(self) -> Self {
        match self {
            Player::Player0 => Player::Player1,
            Player::Player1 => Player::Player0,
        }
    }
}

/// Game status
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GameState {
    Win(Player),
    Tie,
    InProgress,
}

impl Gomoku {
    /// Create a new Gomoku game.
    pub fn new() -> Result<Self, Infallible> {
        Ok(Self {
            board: [[None; 15]; 15],
            next: Player::Player0,
            status: GameState::InProgress,
        })
    }

    /// Get a cell reference from the game board
    /// Panic when target position out of bounds
    pub fn get(&self, row: usize, col: usize) -> &Option<Player> {
        &self.board[row][col]
    }

    /// Check if the game was end
    pub fn is_ended(&self) -> bool {
        self.status != GameState::InProgress
    }

    /// Get the winner of the game. Return `None` when the game is tied or not end yet
    pub fn winner(&self) -> Option<Player> {
        if let GameState::Win(player) = self.status {
            Some(player)
        } else {
            None
        }
    }

    /// Get the game status
    pub fn status(&self) -> &GameState {
        &self.status
    }

    /// Get the next player
    pub fn get_next_player(&self) -> Player {
        self.next
    }

    /// Place a piece on the board
    /// Panic when target position out of bounds
    pub fn place(&mut self, player: Player, row: usize, col: usize) -> Result<(), GomokuError> {
        if self.is_ended() {
            return Err(GomokuError::GameEnded);
        }

        if player != self.next {
            return Err(GomokuError::WrongPlayer);
        }

        if self.board[row][col].is_some() {
            return Err(GomokuError::OccupiedPosition);
        }

        self.board[row][col] = Some(player);
        self.next = self.next.other();

        self.check_state();

        Ok(())
    }

    fn check_state(&mut self) {
        for connectable in Self::get_connectable() {
            let mut last = None;
            let mut count = 0u8;

            for cell in connectable.map(|(row, col)| self.board[col][row]) {
                if cell != last {
                    last = cell;
                    count = 1;
                } else {
                    count += 1;
                    if count == 5 && cell.is_some() {
                        self.status = GameState::Win(cell.unwrap());
                        return;
                    }
                }
            }
        }

        if self.board.iter().flatten().all(|cell| cell.is_some()) {
            self.status = GameState::Tie;
        }
    }

    fn get_connectable() -> impl Iterator<Item = Box<dyn Iterator<Item = (usize, usize)>>> {
        let horizontal = (0usize..15).map(move |row| {
            Box::new((0usize..15).map(move |col| (row, col)))
                as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical = (0usize..15).map(move |col| {
            Box::new((0usize..15).map(move |row| (row, col)))
                as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let horizontal_upper_left_to_lower_right = (0usize..15).map(move |col| {
            Box::new(
                iter::successors(Some((0usize, col)), |(row, col)| Some((row + 1, col + 1)))
                    .take(15 - col),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical_upper_left_to_lower_right = (0usize..15).map(move |row| {
            Box::new(
                iter::successors(Some((row, 0usize)), |(row, col)| Some((row + 1, col + 1)))
                    .take(15 - row),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let horizontal_upper_right_to_lower_left = (0usize..15).map(move |col| {
            Box::new(
                iter::successors(Some((0usize, col)), |(row, col)| {
                    col.checked_sub(1).map(|new_col| (row + 1, new_col))
                })
                .take(1 + col),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical_upper_right_to_lower_left = (0usize..15).map(move |row| {
            Box::new(
                iter::successors(Some((row, 14usize)), |(row, col)| Some((row + 1, col - 1)))
                    .take(15 - row),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        horizontal
            .chain(vertical)
            .chain(horizontal_upper_left_to_lower_right)
            .chain(vertical_upper_left_to_lower_right)
            .chain(horizontal_upper_right_to_lower_left)
            .chain(vertical_upper_right_to_lower_left)
    }
}

/// Errors that can occur when placing a piece on the board
#[derive(Debug, Eq, PartialEq, Snafu)]
pub enum GomokuError {
    #[snafu(display("Wrong player"))]
    WrongPlayer,
    #[snafu(display("Occupied position"))]
    OccupiedPosition,
    #[snafu(display("The game was already end"))]
    GameEnded,
}
#[cfg(test)]
mod tests_llm_16_21 {
    use super::*;

use crate::*;
    use gomoku::{Gomoku, GameState, Player};

    #[test]
    fn test_check_state_win_horizontal() {
        let mut game = Gomoku::new().unwrap();

        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player0, 0, 3).unwrap();
        game.place(Player::Player0, 0, 4).unwrap();

        assert_eq!(game.status, GameState::Win(Player::Player0));
    }

    #[test]
    fn test_check_state_win_vertical() {
        let mut game = Gomoku::new().unwrap();

        game.place(Player::Player1, 0, 0).unwrap();
        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.place(Player::Player1, 3, 0).unwrap();
        game.place(Player::Player1, 4, 0).unwrap();

        assert_eq!(game.status, GameState::Win(Player::Player1));
    }

    #[test]
    fn test_check_state_win_diagonal_up_right() {
        let mut game = Gomoku::new().unwrap();

        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.place(Player::Player0, 3, 3).unwrap();
        game.place(Player::Player0, 4, 4).unwrap();

        assert_eq!(game.status, GameState::Win(Player::Player0));
    }

    #[test]
    fn test_check_state_win_diagonal_down_right() {
        let mut game = Gomoku::new().unwrap();

        game.place(Player::Player1, 4, 0).unwrap();
        game.place(Player::Player1, 3, 1).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        game.place(Player::Player1, 1, 3).unwrap();
        game.place(Player::Player1, 0, 4).unwrap();

        assert_eq!(game.status, GameState::Win(Player::Player1));
    }

    #[test]
    fn test_check_state_tie() {
        let mut game = Gomoku::new().unwrap();

        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 0, 3).unwrap();
        game.place(Player::Player0, 0, 4).unwrap();
        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 1, 2).unwrap();
        game.place(Player::Player0, 1, 3).unwrap();
        game.place(Player::Player1, 1, 4).unwrap();
        game.place(Player::Player0, 2, 0).unwrap();
        game.place(Player::Player1, 2, 1).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.place(Player::Player1, 2, 3).unwrap();
        game.place(Player::Player0, 2, 4).unwrap();
        game.place(Player::Player0, 3, 0).unwrap();
        game.place(Player::Player1, 3, 1).unwrap();
        game.place(Player::Player0, 3, 2).unwrap();
        game.place(Player::Player1, 3, 3).unwrap();
        game.place(Player::Player0, 3, 4).unwrap();
        game.place(Player::Player1, 4, 0).unwrap();
        game.place(Player::Player0, 4, 1).unwrap();
        game.place(Player::Player1, 4, 2).unwrap();
        game.place(Player::Player0, 4, 3).unwrap();
        game.place(Player::Player1, 4, 4).unwrap();

        assert_eq!(game.status, GameState::Tie);
    }
}#[cfg(test)]
mod tests_llm_16_22 {
    use super::*;

use crate::*;
    use std::convert::Infallible;

    #[test]
    fn test_get_valid_position() {
        let mut game = Gomoku::new().unwrap();
        game.place(Player::Player0, 7, 7).unwrap();

        let cell = game.get(7, 7);
        assert_eq!(cell, &Some(Player::Player0));
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 15 but the index is 15")]
    fn test_get_invalid_position() {
        let game = Gomoku::new().unwrap();
        let cell = game.get(15, 15);
    }
}#[cfg(test)]
mod tests_llm_16_24_llm_16_23 {
    use super::*;

use crate::*;
    use crate::gomoku::Gomoku;

    #[test]
    fn test_get_connectable() {
        let connectable = Gomoku::get_connectable();
        let expected_len = 572;

        assert_eq!(connectable.count(), expected_len);
    }
}#[cfg(test)]
mod tests_llm_16_25 {
    use super::*;

use crate::*;
    use gomoku::{Gomoku, Player};

    #[test]
    fn test_get_next_player() {
        let game = Gomoku::new().unwrap();
        assert_eq!(game.get_next_player(), Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_26 {
    use super::*;

use crate::*;
    use gomoku::Gomoku;
    use gomoku::GameState;

    #[test]
    fn test_is_ended_true() {
        let gomoku = Gomoku {
            board: [
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
                [
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                ],
                [
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                    Some(Player::Player1),
                    Some(Player::Player0),
                ],
            ],
            next: Player::Player1,
            status: GameState::Win(Player::Player1),
        };

        assert_eq!(gomoku.is_ended(), true);
    }

    #[test]
    fn test_is_ended_false() {
        let gomoku = Gomoku::new().unwrap();

        assert_eq!(gomoku.is_ended(), false);
    }
}#[cfg(test)]
mod tests_llm_16_27 {
    use super::*;

use crate::*;

    #[test]
    fn test_new() {
        let result = Gomoku::new();
        assert!(result.is_ok());
    }
}#[cfg(test)]
mod tests_llm_16_28 {
    use crate::gomoku::{Gomoku, Player, GameState, GomokuError};

    #[test]
    fn test_place_valid_position() {
        let mut game = Gomoku::new().unwrap();
        let result = game.place(Player::Player0, 5, 5);
        assert_eq!(result, Ok(()));
        assert_eq!(game.get(5, 5), &Some(Player::Player0));
        assert_eq!(game.get_next_player(), Player::Player1);
        assert_eq!(game.is_ended(), false);
        assert_eq!(game.winner(), None);
        assert_eq!(game.status(), &GameState::InProgress);
    }

    #[test]
    fn test_place_invalid_position() {
        let mut game = Gomoku::new().unwrap();
        game.place(Player::Player0, 5, 5).unwrap();

        let result = game.place(Player::Player1, 5, 5);
        assert_eq!(result, Err(GomokuError::OccupiedPosition));
        assert_eq!(game.get(5, 5), &Some(Player::Player0));
        assert_eq!(game.get_next_player(), Player::Player1);
        assert_eq!(game.is_ended(), false);
        assert_eq!(game.winner(), None);
        assert_eq!(game.status(), &GameState::InProgress);
    }

    #[test]
    fn test_place_wrong_player() {
        let mut game = Gomoku::new().unwrap();
        let result = game.place(Player::Player1, 5, 5);
        assert_eq!(result, Err(GomokuError::WrongPlayer));
        assert_eq!(game.get(5, 5), &None);
        assert_eq!(game.get_next_player(), Player::Player0);
        assert_eq!(game.is_ended(), false);
        assert_eq!(game.winner(), None);
        assert_eq!(game.status(), &GameState::InProgress);
    }

    #[test]
    fn test_place_game_ended() {
        let mut game = Gomoku::new().unwrap();

        // Fill the board with pieces
        for row in 0..15 {
            for col in 0..15 {
                game.place(Player::Player0, row, col).unwrap();
                game.place(Player::Player1, row, col).unwrap();
            }
        }

        let result = game.place(Player::Player0, 0, 0);
        assert_eq!(result, Err(GomokuError::GameEnded));
        assert_eq!(game.get(0, 0), &None);
        assert_eq!(game.get_next_player(), Player::Player0);
        assert_eq!(game.is_ended(), true);
        assert_eq!(game.winner(), None);
        assert_eq!(game.status(), &GameState::Tie);
    }
}#[cfg(test)]
mod tests_llm_16_30 {
    use super::*;

use crate::*;

    #[test]
    fn test_status() {
        let mut game = Gomoku::new().unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player0, 0, 0).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player1, 1, 1).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player0, 0, 1).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player1, 1, 0).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player0, 0, 2).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player1, 1, 2).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player0, 0, 3).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player1, 1, 3).unwrap();
        assert_eq!(*game.status(), GameState::InProgress);
        game.place(Player::Player0, 0, 4).unwrap();
        assert_eq!(*game.status(), GameState::Win(Player::Player0));
    }
}#[cfg(test)]
mod tests_llm_16_31 {
    use super::*;

use crate::*;
    use gomoku::{GameState, Gomoku, Player};

    #[test]
    fn test_winner_returns_none_when_game_is_not_ended() {
        let gomoku = Gomoku::new().unwrap();
        let result = gomoku.winner();
        let expected = None;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_winner_returns_none_when_game_is_tied() {
        let mut gomoku = Gomoku::new().unwrap();
        gomoku.place(Player::Player0, 0, 0).unwrap();
        gomoku.place(Player::Player1, 1, 0).unwrap();
        gomoku.place(Player::Player0, 0, 1).unwrap();
        gomoku.place(Player::Player1, 1, 1).unwrap();
        gomoku.place(Player::Player0, 0, 2).unwrap();
        gomoku.place(Player::Player1, 1, 2).unwrap();
        gomoku.place(Player::Player1, 2, 0).unwrap();
        gomoku.place(Player::Player0, 2, 1).unwrap();
        gomoku.place(Player::Player1, 2, 2).unwrap();
        gomoku.place(Player::Player0, 0, 3).unwrap();
        gomoku.place(Player::Player1, 1, 3).unwrap();
        gomoku.place(Player::Player0, 0, 4).unwrap();
        gomoku.place(Player::Player1, 1, 4).unwrap();
        gomoku.place(Player::Player0, 2, 3).unwrap();
        gomoku.place(Player::Player1, 2, 4).unwrap();
        let result = gomoku.winner();
        let expected = None;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_winner_returns_player_when_player_wins() {
        let mut gomoku = Gomoku::new().unwrap();
        gomoku.place(Player::Player0, 0, 0).unwrap();
        gomoku.place(Player::Player1, 1, 0).unwrap();
        gomoku.place(Player::Player0, 0, 1).unwrap();
        gomoku.place(Player::Player1, 1, 1).unwrap();
        gomoku.place(Player::Player0, 0, 2).unwrap();
        gomoku.place(Player::Player1, 1, 2).unwrap();
        gomoku.place(Player::Player0, 0, 3).unwrap();
        gomoku.place(Player::Player1, 1, 3).unwrap();
        gomoku.place(Player::Player0, 0, 4).unwrap();
        let result = gomoku.winner();
        let expected = Some(Player::Player0);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_32 {
    use super::*;

use crate::*;

    #[test]
    fn test_other_player0() {
        assert_eq!(Player::Player0.other(), Player::Player1);
    }

    #[test]
    fn test_other_player1() {
        assert_eq!(Player::Player1.other(), Player::Player0);
    }
}