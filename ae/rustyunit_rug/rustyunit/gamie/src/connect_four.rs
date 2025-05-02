//! Connect Four
//!
//! Check struct [`ConnectFour`](https://docs.rs/gamie/*/gamie/connect_four/struct.ConnectFour.html) for more information
//!
//! # Examples
//!
//! ```rust
//! # fn connect_four() {
//! use gamie::connect_four::{ConnectFour, Player as ConnectFourPlayer};
//!
//! let mut game = ConnectFour::new().unwrap();
//! game.put(ConnectFourPlayer::Player0, 3).unwrap();
//! game.put(ConnectFourPlayer::Player1, 2).unwrap();
//! // ...
//! # }
//! ```

use crate::std_lib::{iter, Box, Index, IndexMut, Infallible};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use snafu::Snafu;

/// Connect Four
///
/// Passing an invalid position to a method will cause panic. Check the target position validity first when dealing with user input
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConnectFour {
    board: [Column; 7],
    next: Player,
    status: GameState,
}

/// The column of the game board.
///
/// This is a vector-like struct. Inner elements can be accessed by using index
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct Column {
    column: [Option<Player>; 6],
    occupied: usize,
}

impl Column {
    fn is_full(&self) -> bool {
        self.occupied == 6
    }

    fn push(&mut self, player: Player) {
        self.column[self.occupied] = Some(player);
        self.occupied += 1;
    }
}

impl Default for Column {
    fn default() -> Self {
        Self {
            column: [None; 6],
            occupied: 0,
        }
    }
}

impl Index<usize> for Column {
    type Output = Option<Player>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.column[index]
    }
}

impl IndexMut<usize> for Column {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.column[index]
    }
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

impl ConnectFour {
    /// Create a new Connect Four game
    pub fn new() -> Result<Self, Infallible> {
        Ok(Self {
            board: Default::default(),
            next: Player::Player0,
            status: GameState::InProgress,
        })
    }

    /// Get a cell reference from the game board
    /// Panic when target position out of bounds
    pub fn get(&self, row: usize, col: usize) -> &Option<Player> {
        &self.board[5 - row][col]
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

    /// Put a piece into the game board
    /// Panic when target position out of bounds
    pub fn put(&mut self, player: Player, col: usize) -> Result<(), ConnectFourError> {
        if self.is_ended() {
            return Err(ConnectFourError::GameEnded);
        }

        if player != self.next {
            return Err(ConnectFourError::WrongPlayer);
        }

        if self.board[col].is_full() {
            return Err(ConnectFourError::ColumnFilled);
        }

        self.board[col].push(player);
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
                    if count == 4 && cell.is_some() {
                        self.status = GameState::Win(cell.unwrap());
                        return;
                    }
                }
            }
        }

        if (0..7).all(|col| self.board[col][5].is_some()) {
            self.status = GameState::Tie;
        }
    }

    fn get_connectable() -> impl Iterator<Item = Box<dyn Iterator<Item = (usize, usize)>>> {
        let horizontal = (0usize..6).map(move |row| {
            Box::new((0usize..7).map(move |col| (row, col)))
                as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical = (0usize..7).map(move |col| {
            Box::new((0usize..6).map(move |row| (row, col)))
                as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let horizontal_upper_left_to_lower_right = (0usize..7).map(move |col| {
            Box::new(
                iter::successors(Some((0usize, col)), |(row, col)| Some((row + 1, col + 1)))
                    .take((7 - col).min(6)),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical_upper_left_to_lower_right = (0usize..6).map(move |row| {
            Box::new(
                iter::successors(Some((row, 0usize)), |(row, col)| Some((row + 1, col + 1)))
                    .take(6 - row),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let horizontal_upper_right_to_lower_left = (0usize..7).map(move |col| {
            Box::new(
                iter::successors(Some((0usize, col)), |(row, col)| {
                    col.checked_sub(1).map(|new_col| (row + 1, new_col))
                })
                .take((1 + col).min(6)),
            ) as Box<dyn Iterator<Item = (usize, usize)>>
        });

        let vertical_upper_right_to_lower_left = (0usize..6).map(move |row| {
            Box::new(
                iter::successors(Some((row, 6usize)), |(row, col)| Some((row + 1, col - 1)))
                    .take(6 - row),
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

/// Errors that can occur when putting a piece into the board
#[derive(Debug, Eq, PartialEq, Snafu)]
pub enum ConnectFourError {
    #[snafu(display("Wrong player"))]
    WrongPlayer,
    #[snafu(display("Filled Column"))]
    ColumnFilled,
    #[snafu(display("The game was already end"))]
    GameEnded,
}

#[cfg(test)]
mod tests {
    use crate::connect_four::*;
    use ntest::timeout;
    #[test]
    #[timeout(3000)]
    #[no_coverage]
    fn test() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.put(Player::Player1, 2).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player1, 1).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player1, 0).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.put(Player::Player1, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player1, 6).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player1, 6).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.put(Player::Player1, 5).unwrap();
        game.put(Player::Player0, 0).unwrap();
        assert_eq!(Some(Player::Player0), game.winner());
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;

use crate::*;
    use connect_four::{Player, Column};

    #[test]
    fn test_default() {
        let column: Column = Column::default();
        assert_eq!(column.column, [None, None, None, None, None, None]);
        assert_eq!(column.occupied, 0);
    }
}#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;

use crate::*;
    use connect_four::Column;
    use connect_four::Player;

    #[test]
    fn test_push() {
        let mut column = Column::default();
        let player = Player::Player0;
        column.push(player);
        assert_eq!(column.column[0], Some(player));
        assert_eq!(column.occupied, 1);
    }
}#[cfg(test)]
mod tests_llm_16_11_llm_16_10 {
    use super::*;

use crate::*;
    use std::convert::Infallible;
    use connect_four::ConnectFourError;
    use connect_four::GameState;
    use connect_four::Player;
    
    #[test]
    fn test_check_state() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));

        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player1, 0).unwrap();
        game.put(Player::Player1, 1).unwrap();
        game.put(Player::Player1, 2).unwrap();
        game.put(Player::Player1, 3).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player1));

        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player1, 1).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player1, 2).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player1, 3).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));

        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::InProgress);

        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player1, 3).unwrap();
        game.put(Player::Player1, 0).unwrap();
        game.put(Player::Player1, 1).unwrap();
        game.put(Player::Player1, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::InProgress);

        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.put(Player::Player0, 4).unwrap();
        game.put(Player::Player0, 5).unwrap();
        game.put(Player::Player1, 6).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Tie);
    }
}#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_get_valid_position() {
        let game = ConnectFour::new().unwrap();
        let player = Player::Player0;
        let col = 2;
        let row = 3;
        
        let result = game.get(row, col);
        let expected = &None;
        
        assert_eq!(result, expected);
    }
    
    #[test]
    #[should_panic]
    fn test_get_invalid_position() {
        let game = ConnectFour::new().unwrap();
        let player = Player::Player0;
        let col = 10;
        let row = 3;
        
        game.get(row, col);
    }
}#[cfg(test)]
mod tests_llm_16_13 {
    use crate::connect_four::ConnectFour;

    #[test]
    fn test_get_connectable() {
        let connectable = ConnectFour::get_connectable();
        let mut count = 0;

        for _ in connectable {
            count += 1;
        }

        assert_eq!(count, 69);
    }
}#[cfg(test)]
mod tests_llm_16_14 {
    use super::*;

use crate::*;
    use connect_four::{GameState, Player};

    #[test]
    fn test_get_next_player() {
        let game = ConnectFour::new().unwrap();
        let next_player = game.get_next_player();
        assert_eq!(next_player, Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_15 {
    use super::*;

use crate::*;
    use std::convert::Infallible;

    #[test]
    fn test_is_ended_game_in_progress() {
        let game = ConnectFour::new().unwrap();
        assert_eq!(false, game.is_ended());
    }

    #[test]
    fn test_is_ended_game_not_in_progress() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        assert_eq!(true, game.is_ended());
    }

    #[test]
    fn test_is_ended_game_tied() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player1, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player1, 1).unwrap();
        game.put(Player::Player1, 2).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        game.put(Player::Player1, 3).unwrap();
        game.put(Player::Player1, 4).unwrap();
        game.put(Player::Player0, 4).unwrap();
        game.put(Player::Player0, 5).unwrap();
        game.put(Player::Player1, 5).unwrap();
        game.put(Player::Player1, 6).unwrap();
        game.put(Player::Player0, 6).unwrap();
        assert_eq!(true, game.is_ended());
    }
}#[cfg(test)]
mod tests_llm_16_16 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let game = ConnectFour::new().unwrap();
        
        // Add assertions here to test the game
        
        // Example assertions:
        // assert_eq!(game.board, [[Player::None; 7]; 6]);
        // assert_eq!(game.next, Player::Player0);
        // assert_eq!(game.status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;

use crate::*;
    
    #[test]
    #[should_panic(expected = "GameEnded")]
    fn test_put_game_ended() {
        let mut game = ConnectFour::new().unwrap();
        game.status = GameState::Win(Player::Player0);
        game.put(Player::Player0, 0);
    }
    
    #[test]
    #[should_panic(expected = "WrongPlayer")]
    fn test_put_wrong_player() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player1, 0);
    }
    
    #[test]
    #[should_panic(expected = "ColumnFilled")]
    fn test_put_column_filled() {
        let mut game = ConnectFour::new().unwrap();
        for _ in 0..6 {
            game.put(Player::Player0, 0);
        }
        game.put(Player::Player0, 0);
    }
    
    #[test]
    fn test_put_success() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        assert_eq!(game.get_next_player(), Player::Player1);
    }
}#[cfg(test)]
mod tests_llm_16_18 {
    use super::*;

use crate::*;
    use connect_four::{GameState, Player};

    #[test]
    fn test_status() {
        let connect_four = ConnectFour::new().unwrap();
        let expected = &GameState::InProgress;
        let result = connect_four.status();
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_19 {
    use super::*;

use crate::*;
    use crate::connect_four::*;

    #[test]
    fn test_winner_none() {
        let game = ConnectFour::new().unwrap();
        assert_eq!(game.winner(), None);
    }

    #[test]
    fn test_winner_some() {
        let mut game = ConnectFour::new().unwrap();
        game.put(Player::Player0, 0).unwrap();
        game.put(Player::Player0, 1).unwrap();
        game.put(Player::Player0, 2).unwrap();
        game.put(Player::Player0, 3).unwrap();
        assert_eq!(game.winner(), Some(Player::Player0));
    }
}#[cfg(test)]
mod tests_llm_16_20 {
    use crate::connect_four::Player;

    #[test]
    fn test_other() {
        assert_eq!(Player::Player0.other(), Player::Player1);
        assert_eq!(Player::Player1.other(), Player::Player0);
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::connect_four::Column;

    #[test]
    fn test_index() {
        let mut p0 = Column::default();
        let p1: usize = 0;

        p0.index(p1);
    }
}
