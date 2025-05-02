//! Tic-Tac-Toe
//!
//! Check struct [`TicTacToe`](https://docs.rs/gamie/*/gamie/tictactoe/struct.TicTacToe.html) for more information
//!
//! # Examples
//!
//! ```rust
//! use gamie::tictactoe::{TicTacToe, Player as TicTacToePlayer};
//!
//! # fn tictactoe() {
//! let mut game = TicTacToe::new().unwrap();
//!
//! game.place(TicTacToePlayer::Player0, 1, 1).unwrap();
//! game.place(TicTacToePlayer::Player1, 0, 0).unwrap();
//!
//! // ...
//!
//! println!("{:?}", game.status());
//! # }
//! ```

use crate::std_lib::Infallible;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use snafu::Snafu;

/// Tic-Tac-Toe
///
/// Passing an invalid position to a method will cause panic. Check the target position validity first when dealing with user input
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TicTacToe {
    board: [[Option<Player>; 3]; 3],
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

impl TicTacToe {
    /// Create a new Tic-Tac-Toe game
    pub fn new() -> Result<Self, Infallible> {
        Ok(Self {
            board: [[None; 3]; 3],
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
    pub fn place(&mut self, player: Player, row: usize, col: usize) -> Result<(), TicTacToeError> {
        if self.is_ended() {
            return Err(TicTacToeError::GameEnded);
        }

        if player != self.next {
            return Err(TicTacToeError::WrongPlayer);
        }

        if self.board[row][col].is_some() {
            return Err(TicTacToeError::OccupiedPosition);
        }

        self.board[row][col] = Some(player);
        self.next = self.next.other();

        self.check_state();

        Ok(())
    }

    fn check_state(&mut self) {
        for row in 0..3 {
            if self.board[row][0].is_some()
                && self.board[row][0] == self.board[row][1]
                && self.board[row][1] == self.board[row][2]
            {
                self.status = GameState::Win(self.board[row][0].unwrap());
                return;
            }
        }

        for col in 0..3 {
            if self.board[0][col].is_some()
                && self.board[0][col] == self.board[1][col]
                && self.board[1][col] == self.board[2][col]
            {
                self.status = GameState::Win(self.board[0][col].unwrap());
                return;
            }
        }

        if self.board[0][0].is_some()
            && self.board[0][0] == self.board[1][1]
            && self.board[1][1] == self.board[2][2]
        {
            self.status = GameState::Win(self.board[0][0].unwrap());
            return;
        }

        if self.board[0][0].is_some()
            && self.board[0][2] == self.board[1][1]
            && self.board[1][1] == self.board[2][0]
        {
            self.status = GameState::Win(self.board[0][2].unwrap());
            return;
        }

        self.status = if self.board.iter().flatten().all(|p| p.is_some()) {
            GameState::Tie
        } else {
            GameState::InProgress
        };
    }
}

/// Errors that can occur when placing a piece on the board
#[derive(Debug, Eq, PartialEq, Snafu)]
pub enum TicTacToeError {
    #[snafu(display("Wrong player"))]
    WrongPlayer,
    #[snafu(display("Occupied position"))]
    OccupiedPosition,
    #[snafu(display("The game was already end"))]
    GameEnded,
}

#[cfg(test)]
mod tests {
    use crate::tictactoe::*;

    #[test]
    fn test() {
        let mut game = TicTacToe::new().unwrap();

        assert_eq!(game.get_next_player(), Player::Player0,);

        assert_eq!(game.place(Player::Player0, 1, 1), Ok(()));

        assert_eq!(game.get_next_player(), Player::Player1,);

        assert_eq!(
            game.place(Player::Player0, 0, 0),
            Err(TicTacToeError::WrongPlayer)
        );

        assert_eq!(game.place(Player::Player1, 1, 0), Ok(()));

        assert_eq!(game.get_next_player(), Player::Player0,);

        assert!(!game.is_ended());

        assert_eq!(
            game.place(Player::Player0, 1, 1),
            Err(TicTacToeError::OccupiedPosition)
        );

        assert_eq!(game.place(Player::Player0, 2, 2), Ok(()));

        assert_eq!(game.status(), &GameState::InProgress);

        assert_eq!(game.place(Player::Player1, 2, 0), Ok(()));

        assert_eq!(game.place(Player::Player0, 0, 0), Ok(()));

        assert!(game.is_ended());

        assert_eq!(game.winner(), Some(Player::Player0));

        assert_eq!(
            game.place(Player::Player0, 0, 2),
            Err(TicTacToeError::GameEnded)
        );

        assert_eq!(game.winner(), Some(Player::Player0));
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_check_state_win_row() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));
    }
    
    #[test]
    fn test_check_state_win_column() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player1, 0, 0).unwrap();
        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player1));
    }
    
    #[test]
    fn test_check_state_win_diagonal1() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player0));
    }
    
    #[test]
    fn test_check_state_win_diagonal2() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player1, 0, 2).unwrap();
        game.place(Player::Player1, 1, 1).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Win(Player::Player1));
    }
    
    #[test]
    fn test_check_state_tie() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 1, 2).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::Tie);
    }
    
    #[test]
    fn test_check_state_in_progress() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.check_state();
        assert_eq!(game.status(), &GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;

use crate::*;
    use std::panic::catch_unwind;

    #[test]
    fn test_get_valid_position() {
        let game = TicTacToe::new().unwrap();
        let result = game.get(0, 0);
        assert_eq!(result, &None);
    }

    #[test]
    #[should_panic]
    fn test_get_invalid_position() {
        let game = TicTacToe::new().unwrap();
        let _result = game.get(3, 3);
    }
}#[cfg(test)]
mod tests_llm_16_74 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_get_next_player() {
        let game = TicTacToe::new().unwrap();
        let next_player = game.get_next_player();
        assert_eq!(next_player, Player::Player0);
    }
}#[cfg(test)]
mod tests_llm_16_75 {
    use super::*;

use crate::*;
    use tictactoe::{GameState, Player, TicTacToe};

    #[test]
    fn test_is_ended_in_progress() {
        let game = TicTacToe::new().unwrap();
        assert_eq!(game.is_ended(), false);
    }

    #[test]
    fn test_is_ended_win() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 1, 1).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        assert_eq!(game.is_ended(), true);
    }

    #[test]
    fn test_is_ended_tie() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 1, 1).unwrap();
        game.place(Player::Player0, 1, 0).unwrap();
        game.place(Player::Player1, 1, 2).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();
        assert_eq!(game.is_ended(), true);
    }
}#[cfg(test)]
mod tests_llm_16_76 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let game = TicTacToe::new().unwrap();
        
        // Assert that the board is initially empty
        for row in game.board.iter() {
            for cell in row.iter() {
                assert_eq!(*cell, None);
            }
        }
        
        // Assert that the next player is Player0
        assert_eq!(game.next, Player::Player0);
        
        // Assert that the game status is InProgress
        assert_eq!(game.status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_77 {
    use crate::tictactoe::*;

    #[test]
    fn test_place_valid_move() {
        let mut game = TicTacToe::new().unwrap();
        let player = Player::Player0;
        let row = 0;
        let col = 0;
        let result = game.place(player, row, col);
        assert_eq!(result, Ok(()));
    }

    #[test]
    #[should_panic]
    fn test_place_invalid_move() {
        let mut game = TicTacToe::new().unwrap();
        let player = Player::Player0;
        let row = 0;
        let col = 0;
        let _ = game.place(player, row, col);
        let result = game.place(player, row, col);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_place_wrong_player() {
        let mut game = TicTacToe::new().unwrap();
        let player0 = Player::Player0;
        let player1 = Player::Player1;
        let row = 0;
        let col = 0;
        let _ = game.place(player0, row, col);
        let result = game.place(player0, row, col);
        assert_eq!(result, Err(TicTacToeError::WrongPlayer));
        let result = game.place(player1, row, col);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_place_occupied_position() {
        let mut game = TicTacToe::new().unwrap();
        let player0 = Player::Player0;
        let player1 = Player::Player1;
        let row = 0;
        let col = 0;
        let _ = game.place(player0, row, col);
        let result = game.place(player1, row, col);
        assert_eq!(result, Err(TicTacToeError::OccupiedPosition));
    }

    #[test]
    fn test_place_game_ended() {
        let mut game = TicTacToe::new().unwrap();
        let player0 = Player::Player0;
        let player1 = Player::Player1;
        let row1 = 0;
        let col1 = 0;
        let row2 = 1;
        let col2 = 0;
        let row3 = 2;
        let col3 = 0;
        let row4 = 0;
        let col4 = 1;
        let row5 = 1;
        let col5 = 1;
        let row6 = 2;
        let col6 = 1;
        let row7 = 0;
        let col7 = 2;
        let row8 = 1;
        let col8 = 2;
        let row9 = 2;
        let col9 = 2;
        let _ = game.place(player0, row1, col1);
        let _ = game.place(player1, row2, col2);
        let _ = game.place(player0, row3, col3);
        let _ = game.place(player1, row4, col4);
        let _ = game.place(player0, row5, col5);
        let _ = game.place(player1, row6, col6);
        let _ = game.place(player0, row7, col7);
        let _ = game.place(player1, row8, col8);
        let result = game.place(player0, row9, col9);
        assert_eq!(result, Err(TicTacToeError::GameEnded));
    }
}#[cfg(test)]
mod tests_llm_16_78 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_status() {
        let mut game = TicTacToe::new().unwrap();
        assert_eq!(*game.status(), GameState::InProgress);

        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 1).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();

        assert_eq!(*game.status(), GameState::InProgress);

        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player0, 2, 2).unwrap();

        assert_eq!(*game.status(), GameState::Win(Player::Player0));
    }
}#[cfg(test)]
mod tests_llm_16_79 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_winner_none() {
        let game = TicTacToe::new().unwrap();
        let result = game.winner();
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_winner_player() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player0, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        let result = game.winner();
        assert_eq!(result, Some(Player::Player0));
    }
    
    #[test]
    fn test_winner_tie() {
        let mut game = TicTacToe::new().unwrap();
        game.place(Player::Player0, 0, 0).unwrap();
        game.place(Player::Player1, 0, 1).unwrap();
        game.place(Player::Player0, 0, 2).unwrap();
        game.place(Player::Player1, 1, 0).unwrap();
        game.place(Player::Player0, 1, 1).unwrap();
        game.place(Player::Player1, 1, 2).unwrap();
        game.place(Player::Player1, 2, 0).unwrap();
        game.place(Player::Player0, 2, 1).unwrap();
        game.place(Player::Player1, 2, 2).unwrap();
        let result = game.winner();
        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_6 {

    use super::*;
    use crate::tictactoe::Player;

    #[test]
    fn test_rug() {
        let mut p0: Player = Player::Player0;
        <Player>::other(p0);
    }
}