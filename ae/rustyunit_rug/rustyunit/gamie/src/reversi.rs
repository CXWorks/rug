//! Reversi
//!
//! Check struct [`Reversi`](https://docs.rs/gamie/*/gamie/reversi/struct.Reversi.html) for more information
//!
//! # Examples
//!
//! ```rust
//! # fn reversi() {
//! use gamie::reversi::{Reversi, Player as ReversiPlayer};
//!
//! let mut game = Reversi::new().unwrap();
//!
//! game.place(ReversiPlayer::Player0, 2, 4).unwrap();
//!
//! // The next player may not be able to place the piece in any position, so check the output of `get_next_player()`
//! assert_eq!(game.get_next_player(), ReversiPlayer::Player1);
//!
//! game.place(ReversiPlayer::Player1, 2, 3).unwrap();
//!
//! // ...
//! # }
//! ```

use crate::std_lib::{iter, Infallible, Ordering};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use snafu::Snafu;

/// Reversi
///
/// Passing an invalid position to a method will cause panic. Check the target position validity first when dealing with user input
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Reversi {
    board: [[Option<Player>; 8]; 8],
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

impl Reversi {
    /// Create a new Reversi game
    pub fn new() -> Result<Self, Infallible> {
        let mut board = [[None; 8]; 8];
        board[3][3] = Some(Player::Player0);
        board[4][4] = Some(Player::Player0);
        board[3][4] = Some(Player::Player1);
        board[4][3] = Some(Player::Player1);

        Ok(Self {
            board,
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
    pub fn place(&mut self, player: Player, row: usize, col: usize) -> Result<(), ReversiError> {
        self.simple_check_position_validity(row, col, player)?;

        let mut flipped = false;

        for dir in Direction::iter() {
            if let Some((to_row, to_col)) =
                self.check_occupied_line_in_direction(row, col, dir, player)
            {
                self.flip(row, col, to_row, to_col, dir, player);
                flipped = true;
            }
        }

        if flipped {
            self.next = player.other();

            if !self.can_player_move(player.other()) {
                self.next = player;

                if !self.can_player_move(player) {
                    self.check_state();
                }
            }

            Ok(())
        } else {
            Err(ReversiError::InvalidPosition)
        }
    }

    /// Check if a position is valid for placing piece
    /// Panic when target position out of bounds
    pub fn check_position_validity(
        &self,
        row: usize,
        col: usize,
        player: Player,
    ) -> Result<(), ReversiError> {
        self.simple_check_position_validity(row, col, player)?;

        if Direction::iter()
            .map(|dir| self.check_occupied_line_in_direction(row, col, dir, player))
            .any(|o| o.is_some())
        {
            Ok(())
        } else {
            Err(ReversiError::InvalidPosition)
        }
    }

    fn simple_check_position_validity(
        &self,
        row: usize,
        col: usize,
        player: Player,
    ) -> Result<(), ReversiError> {
        if self.is_ended() {
            return Err(ReversiError::GameEnded);
        }

        if player != self.next {
            return Err(ReversiError::WrongPlayer);
        }

        if self.board[row][col].is_some() {
            return Err(ReversiError::OccupiedPosition);
        }

        Ok(())
    }

    fn can_player_move(&self, player: Player) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                if self.board[row][col].is_none()
                    && self.check_position_validity(row, col, player).is_ok()
                {
                    return true;
                }
            }
        }

        false
    }

    fn check_state(&mut self) {
        let mut black_count = 0;
        let mut white_count = 0;

        for cell in self.board.iter().flatten().flatten() {
            match cell {
                Player::Player0 => black_count += 1,
                Player::Player1 => white_count += 1,
            }
        }

        self.status = match black_count.cmp(&white_count) {
            Ordering::Less => GameState::Win(Player::Player1),
            Ordering::Equal => GameState::Tie,
            Ordering::Greater => GameState::Win(Player::Player0),
        };
    }

    fn flip(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
        dir: Direction,
        player: Player,
    ) {
        self.iter_positions_in_direction_from(from_row, from_col, dir)
            .take_while(|(row, col)| *row != to_row || *col != to_col)
            .for_each(|(row, col)| {
                self.board[row][col] = Some(player);
            });
    }

    fn check_occupied_line_in_direction(
        &self,
        row: usize,
        col: usize,
        dir: Direction,
        player: Player,
    ) -> Option<(usize, usize)> {
        let mut pos = self.iter_positions_in_direction_from(row, col, dir);

        pos.next();

        let first = if let Some(pos) = pos.next() {
            pos
        } else {
            return None;
        };

        if self.board[first.0][first.1] != Some(player.other()) {
            return None;
        }

        for (row, col) in pos {
            match self.board[row][col] {
                Some(piece) if piece == player.other() => continue,
                Some(_) => return Some((row, col)),
                None => return None,
            }
        }

        None
    }

    fn iter_positions_in_direction_from(
        &self,
        row: usize,
        col: usize,
        dir: Direction,
    ) -> impl Iterator<Item = (usize, usize)> {
        iter::successors(Some((row, col)), move |(row, col)| {
            let (offset_row, offset_col) = dir.as_offset();
            Some((
                (*row as i8 + offset_row) as usize,
                (*col as i8 + offset_col) as usize,
            ))
        })
        .take_while(|(row, col)| *row < 8 && *col < 8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Upper,
    UpperRight,
    Right,
    LowerRight,
    Lower,
    LowerLeft,
    Left,
    UpperLeft,
}

impl Direction {
    fn as_offset(&self) -> (i8, i8) {
        match self {
            Direction::Upper => (-1, 0),
            Direction::UpperRight => (-1, 1),
            Direction::Right => (0, 1),
            Direction::LowerRight => (1, 1),
            Direction::Lower => (1, 0),
            Direction::LowerLeft => (1, -1),
            Direction::Left => (0, -1),
            Direction::UpperLeft => (-1, -1),
        }
    }

    fn iter() -> impl Iterator<Item = Self> {
        [
            Direction::Upper,
            Direction::UpperRight,
            Direction::Right,
            Direction::LowerRight,
            Direction::Lower,
            Direction::LowerLeft,
            Direction::Left,
            Direction::UpperLeft,
        ]
        .into_iter()
    }
}

/// Errors that can occur when placing a piece on the board
#[derive(Debug, Eq, PartialEq, Snafu)]
pub enum ReversiError {
    #[snafu(display("Wrong player"))]
    WrongPlayer,
    #[snafu(display("Position already occupied"))]
    OccupiedPosition,
    #[snafu(display("Invalid position"))]
    InvalidPosition,
    #[snafu(display("The game was already end"))]
    GameEnded,
}

#[cfg(test)]
mod tests {
    use crate::reversi::*;

    #[test]
    fn test() {
        let mut game = Reversi::new().unwrap();

        assert_eq!(game.place(Player::Player0, 2, 4), Ok(()));

        assert_eq!(game.place(Player::Player1, 2, 3), Ok(()));

        assert_eq!(
            game.place(Player::Player1, 2, 6),
            Err(ReversiError::WrongPlayer)
        );

        assert_eq!(
            game.place(Player::Player0, 2, 6),
            Err(ReversiError::InvalidPosition)
        );
    }
}
#[cfg(test)]
mod tests_llm_16_49 {
    use crate::reversi::Direction;

    #[test]
    fn test_as_offset() {
        assert_eq!(Direction::Upper.as_offset(), (-1, 0));
        assert_eq!(Direction::UpperRight.as_offset(), (-1, 1));
        assert_eq!(Direction::Right.as_offset(), (0, 1));
        assert_eq!(Direction::LowerRight.as_offset(), (1, 1));
        assert_eq!(Direction::Lower.as_offset(), (1, 0));
        assert_eq!(Direction::LowerLeft.as_offset(), (1, -1));
        assert_eq!(Direction::Left.as_offset(), (0, -1));
        assert_eq!(Direction::UpperLeft.as_offset(), (-1, -1));
    }
}#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_other() {
        assert_eq!(Player::Player0.other(), Player::Player1);
        assert_eq!(Player::Player1.other(), Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;

use crate::*;
    use reversi::{Direction, GameState, Player};

    #[test]
    fn test_check_occupied_line_in_direction() {
        let mut board = [[None; 8]; 8];
        let reversi = Reversi {
            board,
            next: Player::Player0,
            status: GameState::InProgress,
        };

        let row = 3;
        let col = 3;
        let dir = Direction::Upper;
        let player = Player::Player0;
        let result = reversi.check_occupied_line_in_direction(row, col, dir, player);

        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_llm_16_56 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_check_position_validity_valid_position() {
        let game = Reversi::new().unwrap();
        assert!(game.check_position_validity(3, 2, Player::Player0).is_ok());
    }
    
    #[test]
    fn test_check_position_validity_invalid_position() {
        let game = Reversi::new().unwrap();
        assert!(game.check_position_validity(0, 0, Player::Player0).is_err());
    }
}#[cfg(test)]
mod tests_llm_16_57 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_check_state() {
        let mut game = Reversi::new().unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::InProgress);
    }
    
    #[test]
    fn test_check_state_win() {
        let mut game = Reversi::new().unwrap();
        game.board = [
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
        ];
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));
    }
    
    #[test]
    fn test_check_state_tie() {
        let mut game = Reversi::new().unwrap();
        game.board = [
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
            ],
            [
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
            ],
            [
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
            ],
            [
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
            [
                Some(Player::Player0),
                Some(Player::Player0),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player1),
                Some(Player::Player0),
                Some(Player::Player0),
            ],
        ];
        game.check_state();
        assert_eq!(game.status(), &GameState::Tie);
    }
}#[cfg(test)]
mod tests_llm_16_59 {
    use super::*;

use crate::*;
    use crate::reversi::{Direction, GameState, Player, Reversi};

    #[test]
    fn test_flip() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 3, 2).unwrap();
        game.place(Player::Player1, 3, 1).unwrap();
        game.place(Player::Player1, 3, 0).unwrap();

        game.flip(3, 2, 3, 0, Direction::Left, Player::Player0);

        assert_eq!(&Some(Player::Player0), game.get(3, 2));
        assert_eq!(&Some(Player::Player0), game.get(3, 1));
        assert_eq!(&Some(Player::Player0), game.get(3, 0));
    }
}#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;

use crate::*;

    #[test]
    fn test_get_valid_position() {
        let game = Reversi::new().unwrap();
        let result = game.get(3, 3);
        assert_eq!(result, &Some(Player::Player0));
    }

    #[test]
    #[should_panic]
    fn test_get_invalid_position() {
        let game = Reversi::new().unwrap();
        let _result = game.get(10, 10);
    }

    #[test]
    fn test_get_next_player() {
        let game = Reversi::new().unwrap();
        let result = game.get_next_player();
        assert_eq!(result, Player::Player0);
    }

    #[test]
    fn test_get_status() {
        let game = Reversi::new().unwrap();
        let result = game.status();
        assert_eq!(result, &GameState::InProgress);
    }

    #[test]
    fn test_place_valid_position() {
        let mut game = Reversi::new().unwrap();
        let result = game.place(Player::Player0, 2, 3);
        assert_eq!(result, Ok(()));
    }

    #[test]
    #[should_panic]
    fn test_place_invalid_position() {
        let mut game = Reversi::new().unwrap();
        let _result = game.place(Player::Player0, 10, 10);
    }
}#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;

use crate::*;

    #[test]
    fn test_get_next_player() {
        let game = Reversi::new().unwrap();
        let next_player = game.get_next_player();
        assert_eq!(next_player, Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_63 {
    use crate::reversi::GameState;
    use crate::reversi::Player;
    use crate::reversi::Reversi;
    use crate::reversi::Direction;
    use crate::reversi::ReversiError;

    #[test]
    fn test_is_ended() {
        let reversi = Reversi::new().unwrap();
        assert_eq!(reversi.is_ended(), false);
    }
}#[cfg(test)]
mod tests_llm_16_64 {
    use super::*;

use crate::*;
    use reversi::Direction;

    #[test]
    fn test_iter_positions_in_direction_from() {
        let reversi = Reversi::new().unwrap();
        let result = reversi.iter_positions_in_direction_from(3, 3, Direction::Right).collect::<Vec<_>>();
        assert_eq!(result, vec![(3, 4), (3, 5), (3, 6), (3, 7)]);
    }
}#[cfg(test)]
mod tests_llm_16_65 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let game = Reversi::new().unwrap();
        assert_eq!(game.board[3][3], Some(Player::Player0));
        assert_eq!(game.board[4][4], Some(Player::Player0));
        assert_eq!(game.board[3][4], Some(Player::Player1));
        assert_eq!(game.board[4][3], Some(Player::Player1));
        assert_eq!(game.next, Player::Player0);
        assert_eq!(game.status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_place_valid_position() {
        let mut game = Reversi::new().unwrap();
        let result = game.place(Player::Player0, 2, 3);
        assert!(result.is_ok());
        assert_eq!(game.get(2, 3), &Some(Player::Player0));
        assert_eq!(game.get_next_player(), Player::Player1);
    }
    
    #[test]
    #[should_panic(expected = "Panic message you expect")]
    fn test_place_invalid_position() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 10, 3).unwrap();
    }
    
    #[test]
    fn test_place_flipped() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 2, 3).unwrap();
        assert_eq!(game.get(2, 3), &Some(Player::Player0));
        assert_eq!(game.get(3, 3), &Some(Player::Player0));
        assert_eq!(game.get(4, 3), &Some(Player::Player0));
        assert_eq!(game.get_next_player(), Player::Player1);
    }
    
    #[test]
    fn test_place_not_flipped() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 2, 3).unwrap();
        assert_eq!(game.get(2, 3), &Some(Player::Player0));
        assert_eq!(game.get(3, 3), &Some(Player::Player0));
        assert!(game.place(Player::Player1, 4, 2).is_err());
        assert_eq!(game.get_next_player(), Player::Player0);
    }
    
    #[test]
    fn test_place_end_game() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 2, 3).unwrap();
        game.place(Player::Player1, 3, 2).unwrap();
        game.place(Player::Player0, 3, 3).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        game.place(Player::Player0, 2, 4).unwrap();
        game.place(Player::Player1, 4, 2).unwrap();
        game.place(Player::Player0, 4, 4).unwrap();
        game.place(Player::Player1, 4, 3).unwrap();
        game.place(Player::Player0, 3, 4).unwrap();
        game.place(Player::Player1, 3, 5).unwrap();
        game.place(Player::Player0, 4, 5).unwrap();
        game.place(Player::Player1, 2, 5).unwrap();
        game.place(Player::Player0, 5, 4).unwrap();
        game.place(Player::Player1, 5, 5).unwrap();
        game.place(Player::Player0, 3, 6).unwrap();
        game.place(Player::Player1, 6, 6).unwrap();
        game.place(Player::Player0, 4, 6).unwrap();
        game.place(Player::Player1, 6, 5).unwrap();
        game.place(Player::Player0, 5, 6).unwrap();
        game.place(Player::Player1, 6, 3).unwrap();
        game.place(Player::Player0, 3, 7).unwrap();
        game.place(Player::Player1, 6, 7).unwrap();
        game.place(Player::Player0, 5, 7).unwrap();
        game.place(Player::Player1, 7, 5).unwrap();
        game.place(Player::Player0, 5, 2).unwrap();
        game.place(Player::Player1, 7, 2).unwrap();
        game.place(Player::Player0, 6, 2).unwrap();
        game.place(Player::Player1, 5, 1).unwrap();
        game.place(Player::Player0, 6, 4).unwrap();
        game.place(Player::Player1, 7, 6).unwrap();
        game.place(Player::Player0, 6, 7).unwrap();
        game.place(Player::Player1, 5, 0).unwrap();
        game.place(Player::Player0, 7, 7).unwrap();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));
        assert_eq!(game.winner(), Some(Player::Player0));
        assert!(game.place(Player::Player1, 0, 0).is_err());
        assert_eq!(game.get_next_player(), Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;

use crate::*;
    use crate::reversi::{Player, ReversiError, Reversi};

    #[test]
    fn test_simple_check_position_validity_game_ended() {
        let game = Reversi {
            board: [[None; 8]; 8],
            next: Player::Player0,
            status: GameState::Win(Player::Player0),
        };

        let result = game.simple_check_position_validity(0, 0, Player::Player0);

        assert_eq!(Err(ReversiError::GameEnded), result);
    }

    #[test]
    fn test_simple_check_position_validity_wrong_player() {
        let game = Reversi {
            board: [[None; 8]; 8],
            next: Player::Player0,
            status: GameState::InProgress,
        };

        let result = game.simple_check_position_validity(0, 0, Player::Player1);

        assert_eq!(Err(ReversiError::WrongPlayer), result);
    }

    #[test]
    fn test_simple_check_position_validity_occupied_position() {
        let mut game = Reversi::new().unwrap();
        game.board[0][0] = Some(Player::Player0);

        let result = game.simple_check_position_validity(0, 0, Player::Player0);

        assert_eq!(Err(ReversiError::OccupiedPosition), result);
    }

    #[test]
    fn test_simple_check_position_validity_valid_position() {
        let game = Reversi::new().unwrap();

        let result = game.simple_check_position_validity(0, 0, Player::Player0);

        assert_eq!(Ok(()), result);
    }
}#[cfg(test)]
mod tests_llm_16_68 {
    use crate::reversi::{Reversi, GameState, Player};
    
    #[test]
    fn test_status() {
        let reversi = Reversi::new().unwrap();
        let status = reversi.status();
        assert_eq!(*status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_69 {
    use crate::reversi::{Reversi, GameState, Player};

    #[test]
    fn test_winner_returns_none_when_game_in_progress() {
        let game = Reversi::new().unwrap();
        assert_eq!(game.winner(), None);
    }

    #[test]
    fn test_winner_returns_none_when_game_tied() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 7, 7).unwrap();
        game.place(Player::Player0, 0, 7).unwrap();
        game.place(Player::Player1, 7, 0).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player1, 7, 1).unwrap();
        game.place(Player::Player0, 0, 6).unwrap();
        game.place(Player::Player1, 7, 6).unwrap();
        game.place(Player::Player0, 1, 0).unwrap();
        game.place(Player::Player1, 1, 7).unwrap();
        game.place(Player::Player0, 6, 0).unwrap();
        game.place(Player::Player1, 6, 7).unwrap();
        game.place(Player::Player0, 6, 1).unwrap();
        game.place(Player::Player1, 6, 6).unwrap();
        game.place(Player::Player0, 1, 6).unwrap();
        game.place(Player::Player1, 1, 1).unwrap();
        game.place(Player::Player0, 6, 6).unwrap();
        game.place(Player::Player1, 1, 1).unwrap();
        game.place(Player::Player0, 2, 0).unwrap();
        game.place(Player::Player1, 2, 7).unwrap();
        game.place(Player::Player0, 5, 0).unwrap();
        game.place(Player::Player1, 5, 7).unwrap();
        game.place(Player::Player0, 6, 2).unwrap();
        game.place(Player::Player1, 6, 5).unwrap();
        game.place(Player::Player0, 5, 2).unwrap();
        game.place(Player::Player1, 5, 5).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 2, 1).unwrap();
        game.place(Player::Player0, 2, 6).unwrap();
        game.place(Player::Player1, 2, 6).unwrap();
        game.place(Player::Player0, 5, 1).unwrap();
        game.place(Player::Player1, 5, 1).unwrap();
        game.place(Player::Player0, 5, 6).unwrap();
        game.place(Player::Player1, 5, 6).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.place(Player::Player1, 2, 5).unwrap();
        game.place(Player::Player0, 5, 2).unwrap();
        game.place(Player::Player1, 5, 5).unwrap();
        game.place(Player::Player0, 3, 0).unwrap();
        game.place(Player::Player1, 3, 7).unwrap();
        game.place(Player::Player0, 4, 0).unwrap();
        game.place(Player::Player1, 4, 7).unwrap();
        game.place(Player::Player0, 6, 3).unwrap();
        game.place(Player::Player1, 6, 4).unwrap();
        game.place(Player::Player0, 5, 3).unwrap();
        game.place(Player::Player1, 5, 4).unwrap();
        game.place(Player::Player0, 3, 1).unwrap();
        game.place(Player::Player1, 3, 1).unwrap();
        game.place(Player::Player0, 3, 6).unwrap();
        game.place(Player::Player1, 3, 6).unwrap();
        game.place(Player::Player0, 4, 1).unwrap();
        game.place(Player::Player1, 4, 1).unwrap();
        game.place(Player::Player0, 4, 6).unwrap();
        game.place(Player::Player1, 4, 6).unwrap();
        game.place(Player::Player0, 3, 2).unwrap();
        game.place(Player::Player1, 3, 5).unwrap();
        game.place(Player::Player0, 4, 2).unwrap();
        game.place(Player::Player1, 4, 5).unwrap();
        game.place(Player::Player0, 3, 3).unwrap();
        game.place(Player::Player1, 3, 4).unwrap();
        game.place(Player::Player0, 4, 3).unwrap();
        game.place(Player::Player1, 4, 4).unwrap();
        game.place(Player::Player0, 2, 3).unwrap();
        game.place(Player::Player1, 2, 4).unwrap();
        game.place(Player::Player0, 5, 3).unwrap();
        game.place(Player::Player1, 5, 4).unwrap();
        game.place(Player::Player0, 3, 5).unwrap();
        game.place(Player::Player1, 3, 3).unwrap();
        game.place(Player::Player0, 3, 4).unwrap();
        game.place(Player::Player1, 3, 4).unwrap();
        game.place(Player::Player0, 2, 4).unwrap();
        game.place(Player::Player1, 2, 3).unwrap();
        game.place(Player::Player0, 5, 3).unwrap();
        game.place(Player::Player1, 5, 4).unwrap();
        game.place(Player::Player0, 4, 3).unwrap();
        game.place(Player::Player1, 4, 4).unwrap();
        game.place(Player::Player0, 3, 5).unwrap();
        game.place(Player::Player1, 3, 2).unwrap();
        game.place(Player::Player0, 4, 5).unwrap();
        game.place(Player::Player1, 4, 2).unwrap();
        game.place(Player::Player0, 2, 5).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        game.place(Player::Player0, 5, 5).unwrap();
        game.place(Player::Player1, 5, 2).unwrap();
        game.place(Player::Player0, 2, 5).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        game.place(Player::Player0, 5, 5).unwrap();
        game.place(Player::Player1, 5, 2).unwrap();
        game.place(Player::Player0, 2, 0).unwrap();
        game.place(Player::Player1, 2, 7).unwrap();
        game.place(Player::Player0, 5, 0).unwrap();
        game.place(Player::Player1, 5, 7).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 7, 2).unwrap();
        game.place(Player::Player0, 0, 5).unwrap();
        game.place(Player::Player1, 7, 5).unwrap();
        game.place(Player::Player0, 2, 0).unwrap();
        game.place(Player::Player1, 5, 0).unwrap();
        game.place(Player::Player0, 2, 7).unwrap();
        game.place(Player::Player1, 5, 7).unwrap();
        game.place(Player::Player0, 0, 3).unwrap();
        game.place(Player::Player1, 7, 3).unwrap();
        game.place(Player::Player0, 0, 4).unwrap();
        game.place(Player::Player1, 7, 4).unwrap();
        game.place(Player::Player0, 3, 0).unwrap();
        game.place(Player::Player1, 3, 7).unwrap();
        game.place(Player::Player0, 4, 0).unwrap();
        game.place(Player::Player1, 4, 7).unwrap();
        game.place(Player::Player0, 3, 7).unwrap();
        game.place(Player::Player1, 3, 0).unwrap();
        game.place(Player::Player0, 4, 7).unwrap();
        game.place(Player::Player1, 4, 0).unwrap();
        game.place(Player::Player0, 3, 1).unwrap();
        game.place(Player::Player1, 3, 6).unwrap();
        game.place(Player::Player0, 4, 1).unwrap();
        game.place(Player::Player1, 4, 6).unwrap();
        game.place(Player::Player0, 1, 3).unwrap();
        game.place(Player::Player1, 6, 3).unwrap();
        game.place(Player::Player0, 1, 4).unwrap();
        game.place(Player::Player1, 6, 4).unwrap();
        game.place(Player::Player0, 1, 3).unwrap();
        game.place(Player::Player1, 6, 3).unwrap();
        game.place(Player::Player0, 1, 4).unwrap();
        game.place(Player::Player1, 6, 4).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 5, 6).unwrap();
        game.place(Player::Player0, 2, 6).unwrap();
        game.place(Player::Player1, 5, 1).unwrap();
        game.place(Player::Player0, 1, 2).unwrap();
        game.place(Player::Player1, 6, 5).unwrap();
        game.place(Player::Player0, 1, 5).unwrap();
        game.place(Player::Player1, 6, 2).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 5, 6).unwrap();
        game.place(Player::Player0, 6, 2).unwrap();
        game.place(Player::Player1, 1, 5).unwrap();
        game.place(Player::Player0, 5, 2).unwrap();
        game.place(Player::Player1, 2, 5).unwrap();
        game.place(Player::Player0, 6, 5).unwrap();
        game.place(Player::Player1, 1, 2).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 6, 6).unwrap();
        game.place(Player::Player0, 1, 6).unwrap();
        game.place(Player::Player1, 6, 1).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 6, 6).unwrap();
        game.place(Player::Player0, 6, 1).unwrap();
        game.place(Player::Player1, 1, 6).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player1, 7, 6).unwrap();
        game.place(Player::Player0, 0, 6).unwrap();
        game.place(Player::Player1, 7, 1).unwrap();
        game.place(Player::Player0, 1, 0).unwrap();
        game.place(Player::Player1, 6, 7).unwrap();
        game.place(Player::Player0, 1, 7).unwrap();
        game.place(Player::Player1, 6, 0).unwrap();
        game.place(Player::Player0, 1, 0).unwrap();
        game.place(Player::Player1, 6, 7).unwrap();
        game.place(Player::Player0, 6, 0).unwrap();
        game.place(Player::Player1, 1, 7).unwrap();

        assert_eq!(game.winner(), None);
    }

    #[test]
    fn test_winner_returns_winner_player_when_game_won() {
        let mut game = Reversi::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 7).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player1, 0, 6).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 0, 5).unwrap();
        game.place(Player::Player0, 0, 3).unwrap();
        game.place(Player::Player1, 0, 4).unwrap();
        game.place(Player::Player0, 1, 0).unwrap();
        game.place(Player::Player1, 1, 7).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 1, 6).unwrap();
        game.place(Player::Player0, 1, 2).unwrap();
        game.place(Player::Player1, 1, 5).unwrap();
        game.place(Player::Player0, 1, 3).unwrap();
        game.place(Player::Player1, 1, 4).unwrap();
        game.place(Player::Player0, 2, 0).unwrap();
        game.place(Player::Player1, 2, 7).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 2, 6).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.place(Player::Player1, 2, 5).unwrap();
        game.place(Player::Player0, 2, 3).unwrap();
        game.place(Player::Player1, 2, 4).unwrap();
        game.place(Player::Player0, 3, 0).unwrap();
        game.place(Player::Player1, 3, 7).unwrap();
        game.place(Player::Player0, 3, 1).unwrap();
        game.place(Player::Player1, 3, 6).unwrap();
        game.place(Player::Player0, 3, 2).unwrap();
        game.place(Player::Player1, 3, 5).unwrap();
        game.place(Player::Player0, 3, 3).unwrap();
        game.place(Player::Player1, 3, 4).unwrap();
        game.place(Player::Player0, 4, 0).unwrap();
        game.place(Player::Player1, 4, 7).unwrap();
        game.place(Player::Player0, 4, 1).unwrap();
        game.place(Player::Player1, 4, 6).unwrap();
        game.place(Player::Player0, 4, 2).unwrap();
        game.place(Player::Player1, 4, 5).unwrap();
        game.place(Player::Player0, 4, 3).unwrap();
        game.place(Player::Player1, 4, 4).unwrap();
        game.place(Player::Player0, 5, 0).unwrap();
        game.place(Player::Player1, 5, 7).unwrap();
        game.place(Player::Player0, 5, 1).unwrap();
        game.place(Player::Player1, 5, 6).unwrap();
        game.place(Player::Player0, 5, 2).unwrap();
        game.place(Player::Player1, 5, 5).unwrap();
        game.place(Player::Player0, 5, 3).unwrap();
        game.place(Player::Player1, 5, 4).unwrap();
        game.place(Player::Player0, 6, 0).unwrap();
        game.place(Player::Player1, 6, 7).unwrap();
        game.place(Player::Player0, 6, 1).unwrap();
        game.place(Player::Player1, 6, 6).unwrap();
        game.place(Player::Player0, 6, 2).unwrap();
        game.place(Player::Player1, 6, 5).unwrap();
        game.place(Player::Player0, 6, 3).unwrap();
        game.place(Player::Player1, 6, 4).unwrap();
        game.place(Player::Player0, 7, 0).unwrap();
        game.place(Player::Player1, 7, 7).unwrap();
        game.place(Player::Player0, 7, 1).unwrap();
        game.place(Player::Player1, 7, 6).unwrap();
        game.place(Player::Player0, 7, 2).unwrap();
        game.place(Player::Player1, 7, 5).unwrap();
        game.place(Player::Player0, 7, 3).unwrap();
        game.place(Player::Player1, 7, 4).unwrap();
        game.place(Player::Player0, 7, 4).unwrap();

        assert_eq!(game.winner(), Some(Player::Player0));
    }
}#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::reversi::{Reversi, Player};

    #[test]
    fn test_can_player_move() {
        let mut p0: Reversi = Reversi::new().unwrap();
        let mut p1: Player = Player::Player0;

        p0.can_player_move(p1);
    }
}#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::reversi::Direction;
    
    #[test]
    fn test_rug() {
        Direction::iter();
    }
}