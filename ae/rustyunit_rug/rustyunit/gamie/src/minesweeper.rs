//! Minesweeper
//!
//! Check struct [`Minesweeper`](https://docs.rs/gamie/*/gamie/minesweeper/struct.Minesweeper.html) for more information
//!
//! # Examples
//!
//! ```rust
//! # fn minesweeper() {
//! use gamie::minesweeper::Minesweeper;
//! use rand::rngs::ThreadRng;
//!
//! let mut game = Minesweeper::new(8, 8, 9, &mut ThreadRng::default()).unwrap();
//!
//! game.toggle_flag(3, 2).unwrap();
//! // ...
//! game.click(7, 7, true).unwrap();
//! // ...
//! # }
//! ```

use crate::std_lib::{iter, Vec, VecDeque};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use rand::{
    distributions::{Distribution, Uniform},
    Rng,
};
use snafu::Snafu;

/// Minesweeper
///
/// To avoid unessecary memory allocation, the game board is stored in a single `Vec` rather than a nested one.
///
/// Passing an invalid position to a method will cause panic. Check the target position validity first when dealing with user input
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Minesweeper {
    board: Vec<Cell>,
    height: usize,
    width: usize,
    status: GameState,
}

/// The cell in the board.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Cell {
    pub is_mine: bool,
    pub mine_adjacent: usize,
    pub is_revealed: bool,
    pub is_flagged: bool,
}

impl Cell {
    fn new(is_mine: bool) -> Self {
        Self {
            is_mine,
            mine_adjacent: 0,
            is_revealed: false,
            is_flagged: false,
        }
    }
}

/// Game status
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GameState {
    Win,
    Exploded(Vec<(usize, usize)>),
    InProgress,
}

impl Minesweeper {
    /// Create a new Minesweeper game
    ///
    /// A mutable reference of a random number generator is required for randomizing mine positions
    ///
    /// Return `Err(MinesweeperError::TooManyMines)` if `height * width < mines`
    ///
    /// # Examples
    /// ```rust
    /// # fn minesweeper() {
    /// use gamie::minesweeper::Minesweeper;
    /// use rand::rngs::ThreadRng;
    ///
    /// let mut game = Minesweeper::new(8, 8, 9, &mut ThreadRng::default()).unwrap();
    /// # }
    /// ```
    pub fn new<R: Rng + ?Sized>(
        height: usize,
        width: usize,
        mines: usize,
        rng: &mut R,
    ) -> Result<Self, MinesweeperError> {
        if height * width < mines {
            return Err(MinesweeperError::TooManyMines);
        }

        let board = iter::repeat(Cell::new(true))
            .take(mines)
            .chain(iter::repeat(Cell::new(false)).take(height * width - mines))
            .collect();

        let mut minesweeper = Self {
            board,
            height,
            width,
            status: GameState::InProgress,
        };
        minesweeper.randomize(rng).unwrap();

        Ok(minesweeper)
    }

    /// Randomize the board
    ///
    /// A mutable reference of a random number generator is required for randomizing mine positions
    pub fn randomize<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), MinesweeperError> {
        if self.is_ended() {
            return Err(MinesweeperError::GameEnded);
        }

        let range = Uniform::from(0..self.height * self.width);

        for idx in 0..self.height * self.width {
            self.board.swap(idx, range.sample(rng));
        }

        self.update_around_mine_count();

        Ok(())
    }

    /// Get a cell reference from the game board
    /// Panic when target position out of bounds
    pub fn get(&self, row: usize, col: usize) -> &Cell {
        if row >= self.height || col >= self.width {
            panic!("Invalid position: ({}, {})", row, col);
        }

        &self.board[row * self.width + col]
    }

    /// Check if the game was end
    pub fn is_ended(&self) -> bool {
        self.status != GameState::InProgress
    }

    /// Get the game status
    pub fn status(&self) -> &GameState {
        &self.status
    }

    /// Click a cell on the game board
    ///
    /// Clicking an already revealed cell will unreveal its adjacent cells if the flagged cell count around it equals to its adjacent mine count
    /// When `auto_flag` is `true`, clicking an already revealed cell will flag its adjacent unflagged-unrevealed cells if the unflagged-revealed cell count around it equals to its adjacent mine count
    ///
    /// The return value indicates if the game board is changed from the click
    ///
    /// Panic when target position out of bounds
    pub fn click(
        &mut self,
        row: usize,
        col: usize,
        auto_flag: bool,
    ) -> Result<bool, MinesweeperError> {
        if row >= self.height || col >= self.width {
            panic!("Invalid position: ({}, {})", row, col);
        }

        if self.is_ended() {
            return Err(MinesweeperError::GameEnded);
        }

        if !self.board[row * self.width + col].is_revealed {
            self.click_unrevealed(row, col)?;
            Ok(true)
        } else {
            Ok(self.click_revealed(row, col, auto_flag)?)
        }
    }

    /// Flag or unflag a cell on the board
    /// Return Err(MinesweeperError::AlreadyRevealed) if the target cell is already revealed
    ///
    /// Panic when target position out of bounds
    pub fn toggle_flag(&mut self, row: usize, col: usize) -> Result<(), MinesweeperError> {
        if row >= self.height || col >= self.width {
            panic!("Invalid position: ({}, {})", row, col);
        }

        if self.is_ended() {
            return Err(MinesweeperError::GameEnded);
        }

        if self.board[row * self.width + col].is_revealed {
            return Err(MinesweeperError::AlreadyRevealed);
        }

        self.board[row * self.width + col].is_flagged =
            !self.board[row * self.width + col].is_flagged;

        self.check_state();

        Ok(())
    }

    fn click_unrevealed(&mut self, row: usize, col: usize) -> Result<(), MinesweeperError> {
        if self.board[row * self.width + col].is_flagged {
            return Err(MinesweeperError::AlreadyFlagged);
        }

        if self.board[row * self.width + col].is_mine {
            self.status = GameState::Exploded(vec![(row, col)]);
            return Ok(());
        }

        self.reveal_from(row * self.width + col);
        self.check_state();

        Ok(())
    }

    fn click_revealed(
        &mut self,
        row: usize,
        col: usize,
        auto_flag: bool,
    ) -> Result<bool, MinesweeperError> {
        let mut is_changed = false;

        if self.board[row * self.width + col].mine_adjacent > 0 {
            let mut adjacent_all = 0;
            let mut adjacent_revealed = 0;
            let mut adjacent_flagged = 0;

            self.get_adjacent_cells(row, col)
                .map(|idx| self.board[idx])
                .for_each(|cell| {
                    adjacent_all += 1;

                    if cell.is_revealed {
                        adjacent_revealed += 1;
                    } else if cell.is_flagged {
                        adjacent_flagged += 1;
                    }
                });

            let adjacent_unrevealed = adjacent_all - adjacent_revealed - adjacent_flagged;

            if adjacent_unrevealed > 0 {
                if adjacent_flagged == self.board[row * self.width + col].mine_adjacent {
                    let mut exploded = None;

                    self.get_adjacent_cells(row, col).for_each(|idx| {
                        if !self.board[idx].is_flagged && !self.board[idx].is_revealed {
                            if self.board[idx].is_mine {
                                self.board[idx].is_revealed = true;

                                match exploded {
                                    None => exploded = Some(vec![(row, col)]),
                                    Some(ref mut exploded) => {
                                        exploded.push((row, col));
                                    }
                                }
                            } else {
                                self.reveal_from(idx);
                                is_changed = true;
                            }
                        }
                    });

                    if let Some(exploded) = exploded {
                        self.status = GameState::Exploded(exploded);
                        return Ok(true);
                    }
                }

                if auto_flag
                    && adjacent_unrevealed + adjacent_flagged
                        == self.board[row * self.width + col].mine_adjacent
                {
                    self.get_adjacent_cells(row, col).for_each(|idx| {
                        if !self.board[idx].is_flagged && !self.board[idx].is_revealed {
                            self.board[idx].is_flagged = true;
                            is_changed = true;
                        }
                    });
                }
            }

            self.check_state();
        }

        Ok(is_changed)
    }

    fn reveal_from(&mut self, idx: usize) {
        if self.board[idx].mine_adjacent != 0 {
            self.board[idx].is_revealed = true;
        } else {
            let mut cell_idxs_to_reveal = VecDeque::new();
            cell_idxs_to_reveal.push_back(idx);

            while let Some(cell_idx) = cell_idxs_to_reveal.pop_front() {
                self.board[cell_idx].is_revealed = true;

                for neighbor_idx in
                    self.get_adjacent_cells(cell_idx / self.width, cell_idx % self.width)
                {
                    if !self.board[neighbor_idx].is_flagged && !self.board[neighbor_idx].is_revealed
                    {
                        if self.board[neighbor_idx].mine_adjacent == 0 {
                            cell_idxs_to_reveal.push_back(neighbor_idx);
                        } else {
                            self.board[neighbor_idx].is_revealed = true;
                        }
                    }
                }
            }
        }
    }

    fn check_state(&mut self) {
        self.status = if self
            .board
            .iter()
            .filter(|cell| !cell.is_mine)
            .all(|cell| cell.is_revealed)
        {
            GameState::Win
        } else {
            GameState::InProgress
        };
    }

    fn update_around_mine_count(&mut self) {
        for idx in 0..self.height * self.width {
            let count = self
                .get_adjacent_cells(idx / self.width, idx % self.width)
                .filter(|idx| self.board[*idx].is_mine)
                .count();

            self.board[idx].mine_adjacent = count;
        }
    }

    fn get_adjacent_cells(&self, row: usize, col: usize) -> AdjacentCells {
        AdjacentCells::new(row, col, self.height, self.width)
    }
}

#[derive(Clone)]
struct AdjacentCells {
    around: [(isize, isize); 8],
    board_height: isize,
    board_width: isize,
    offset: usize,
}

impl Iterator for AdjacentCells {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.around[self.offset..]
            .iter()
            .enumerate()
            .filter(|(_, (row, col))| {
                *row >= 0 && *col >= 0 && *row < self.board_height && *col < self.board_width
            })
            .next()
            .map(|(idx, (row, col))| {
                self.offset += idx + 1;
                (row * self.board_width + col) as usize
            })
    }
}

impl AdjacentCells {
    fn new(row: usize, col: usize, board_height: usize, board_width: usize) -> Self {
        let (row, col, board_height, board_width) = (
            row as isize,
            col as isize,
            board_height as isize,
            board_width as isize,
        );

        AdjacentCells {
            around: [
                (row - 1, col - 1),
                (row - 1, col),
                (row - 1, col + 1),
                (row, col - 1),
                (row, col + 1),
                (row + 1, col - 1),
                (row + 1, col),
                (row + 1, col + 1),
            ],
            board_height,
            board_width,
            offset: 0,
        }
    }
}

/// Errors that can occur.
#[derive(Debug, Eq, PartialEq, Snafu)]
pub enum MinesweeperError {
    #[snafu(display("Too many mines"))]
    TooManyMines,
    #[snafu(display("Clicked an already flagged cell"))]
    AlreadyFlagged,
    #[snafu(display("Clicked an already revealed cell"))]
    AlreadyRevealed,
    #[snafu(display("The game was already end"))]
    GameEnded,
}
#[cfg(test)]
mod tests_llm_16_6 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_next() {
        let mut adjacent_cells = AdjacentCells::new(1, 1, 3, 3);
        assert_eq!(adjacent_cells.next(), Some(0));
        assert_eq!(adjacent_cells.next(), Some(1));
        assert_eq!(adjacent_cells.next(), Some(2));
        assert_eq!(adjacent_cells.next(), Some(3));
        assert_eq!(adjacent_cells.next(), Some(5));
        assert_eq!(adjacent_cells.next(), Some(6));
        assert_eq!(adjacent_cells.next(), Some(7));
        assert_eq!(adjacent_cells.next(), None);
    }
}#[cfg(test)]
mod tests_llm_16_33 {
    use crate::minesweeper::AdjacentCells;

    #[test]
    fn test_new() {
        let row = 2;
        let col = 3;
        let board_height = 5;
        let board_width = 5;

        let result = AdjacentCells::new(row, col, board_height, board_width);

        let expected_around = [
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 2),
            (2, 4),
            (3, 2),
            (3, 3),
            (3, 4),
        ];
        let expected_board_height = 5;
        let expected_board_width = 5;
        let expected_offset = 0;

        let expected = (expected_around, expected_board_height, expected_board_width, expected_offset);

        assert_eq!(result.around, expected.0);
        assert_eq!(result.board_height, expected.1);
        assert_eq!(result.board_width, expected.2);
        assert_eq!(result.offset, expected.3);
    }
}#[cfg(test)]
mod tests_llm_16_34 {
    use crate::minesweeper::Cell;

    #[test]
    fn test_new() {
        let cell = Cell::new(true);

        assert_eq!(cell.is_mine, true);
        assert_eq!(cell.mine_adjacent, 0);
        assert_eq!(cell.is_revealed, false);
        assert_eq!(cell.is_flagged, false);
    }
}#[cfg(test)]
mod tests_llm_16_35 {
    use super::*;

use crate::*;
    use crate::minesweeper::GameState;

    #[test]
    fn check_state_all_cells_revealed() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_mine: false,
                    is_revealed: true,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: true,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: true,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
            ],
            height: 3,
            width: 1,
            status: GameState::InProgress,
        };

        minesweeper.check_state();

        assert_eq!(minesweeper.status, GameState::Win);
    }

    #[test]
    fn check_state_not_all_cells_revealed() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_mine: false,
                    is_revealed: true,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
            ],
            height: 3,
            width: 1,
            status: GameState::InProgress,
        };

        minesweeper.check_state();

        assert_eq!(minesweeper.status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_37_llm_16_36 {
    use crate::minesweeper::{Minesweeper, GameState, MinesweeperError};
    use rand::Rng;

    #[test]
    #[should_panic(expected = "Invalid position")]
    fn test_click_invalid_position() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.click(10, 10, false).unwrap();
    }

    #[test]
    fn test_click_game_ended() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.status = GameState::Win;
        let result = game.click(0, 0, false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), MinesweeperError::GameEnded);
    }

    #[test]
    fn test_click_unrevealed() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        let result = game.click(0, 0, false);
        assert!(result.unwrap());
    }

    #[test]
    fn test_click_revealed() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.reveal_from(0);
        let result = game.click(0, 0, false);
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    #[should_panic(expected = "Invalid position")]
    fn test_toggle_flag_invalid_position() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.toggle_flag(10, 10).unwrap();
    }

    #[test]
    fn test_toggle_flag_game_ended() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.status = GameState::Exploded(vec![]);
        let result = game.toggle_flag(0, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), MinesweeperError::GameEnded);
    }

    #[test]
    fn test_toggle_flag_already_revealed() {
        let mut game = Minesweeper::new(8, 8, 9, &mut rand::thread_rng()).unwrap();
        game.reveal_from(0);
        let result = game.toggle_flag(0, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), MinesweeperError::AlreadyRevealed);
    }
}#[cfg(test)]
mod tests_llm_16_38 {
    use super::*;

use crate::*;
    use crate::minesweeper::Cell;

    #[test]
    fn test_click_revealed_returns_false_when_no_changes_to_board() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell::new(true),
                Cell::new(true),
                Cell::new(true),
                Cell {
                    mine_adjacent: 0,
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                },
            ],
            height: 2,
            width: 2,
            status: GameState::InProgress,
        };

        let result = minesweeper.click_revealed(1, 1, false);

        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_click_revealed_reveals_adjacent_cells() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell::new(true),
                Cell::new(true),
                Cell {
                    mine_adjacent: 0,
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                },
                Cell {
                    mine_adjacent: 0,
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                },
            ],
            height: 2,
            width: 2,
            status: GameState::InProgress,
        };

        let result = minesweeper.click_revealed(0, 0, false);

        assert_eq!(result.unwrap(), true);
        assert_eq!(minesweeper.board[2].is_revealed, true);
        assert_eq!(minesweeper.board[3].is_revealed, true);
    }

    #[test]
    fn test_click_revealed_reveals_no_cells_when_adjacent_cells_flagged() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell::new(true),
                Cell::new(true),
                Cell {
                    mine_adjacent: 1,
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                },
                Cell {
                    mine_adjacent: 1,
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                },
            ],
            height: 2,
            width: 2,
            status: GameState::InProgress,
        };

        let result = minesweeper.click_revealed(0, 0, false);

        assert_eq!(result.unwrap(), false);
        assert_eq!(minesweeper.board[2].is_revealed, false);
        assert_eq!(minesweeper.board[3].is_revealed, false);
    }
}#[cfg(test)]
mod tests_llm_16_39 {
    use super::*;

use crate::*;

    #[test]
    fn test_click_unrevealed_already_flagged() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_revealed: false,
                    is_flagged: true,
                    is_mine: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
            ],
            height: 1,
            width: 3,
            status: GameState::InProgress,
        };

        assert_eq!(
            minesweeper
                .click_unrevealed(0, 0)
                .unwrap_err(),
            MinesweeperError::AlreadyFlagged
        );
    }

    #[test]
    fn test_click_unrevealed_mine() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: true,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
            ],
            height: 1,
            width: 3,
            status: GameState::InProgress,
        };

        assert_eq!(
            minesweeper
                .click_unrevealed(0, 0)
                .unwrap(),
            ()
        );

        assert_eq!(
            minesweeper.status,
            GameState::Exploded(vec![(0, 0)])
        );
    }

    #[test]
    fn test_click_unrevealed_success() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_revealed: false,
                    is_flagged: false,
                    is_mine: false,
                    mine_adjacent: 0,
                },
            ],
            height: 1,
            width: 3,
            status: GameState::InProgress,
        };

        assert_eq!(
            minesweeper
                .click_unrevealed(0, 0)
                .unwrap(),
            ()
        );

        assert_eq!(
            minesweeper.status,
            GameState::InProgress
        );
    }
}#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;

use crate::*;
    use rand::rngs::ThreadRng;

    #[test]
    #[should_panic(expected = "Invalid position: (9, 9)")]
    fn test_get_invalid_position() {
        let height = 8;
        let width = 8;
        let mines = 9;
        let mut rng = ThreadRng::default();
        let minesweeper = Minesweeper::new(height, width, mines, &mut rng).unwrap();

        let _ = minesweeper.get(9, 9);
    }

    #[test]
    fn test_get_valid_position() {
        let height = 8;
        let width = 8;
        let mines = 9;
        let mut rng = ThreadRng::default();
        let minesweeper = Minesweeper::new(height, width, mines, &mut rng).unwrap();

        let cell = minesweeper.get(5, 5);
        assert_eq!(cell.is_mine, false);
    }
}#[cfg(test)]
mod tests_llm_16_41 {
    use super::*;

use crate::*;
    use rand::rngs::mock::StepRng;

    #[test]
    fn test_get_adjacent_cells() {
        let mut rng = StepRng::new(1, 1);
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();

        let expected_0_0 = [1, 8, 9];
        assert_eq!(
            minesweeper.get_adjacent_cells(0, 0).collect::<Vec<usize>>(),
            expected_0_0
        );

        let expected_3_3 = [10, 11, 12, 19, 21, 28, 29, 30];
        assert_eq!(
            minesweeper.get_adjacent_cells(3, 3).collect::<Vec<usize>>(),
            expected_3_3
        );

        let expected_7_7 = [46, 54, 55];
        assert_eq!(
            minesweeper.get_adjacent_cells(7, 7).collect::<Vec<usize>>(),
            expected_7_7
        );
    }
}
#[cfg(test)]
mod tests_llm_16_42 {
    use super::*;

use crate::*;
    use crate::minesweeper::GameState;

    #[test]
    fn test_is_ended_game_in_progress() {
        let minesweeper = Minesweeper::new(10, 10, 10, &mut rand::thread_rng()).unwrap();
        assert_eq!(minesweeper.is_ended(), false);
    }

    #[test]
    fn test_is_ended_game_ended() {
        let board = vec![
            Cell {
                is_revealed: true,
                is_mine: false,
                is_flagged: false,
                mine_adjacent: 0,
            };
            10 * 10
        ];

        let minesweeper = Minesweeper {
            board,
            height: 10,
            width: 10,
            status: GameState::Win,
        };

        assert_eq!(minesweeper.is_ended(), true);
    }
}
#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;

use crate::*;
    use rand::rngs::ThreadRng;

    #[test]
    fn test_new_success() {
        let mut rng = ThreadRng::default();
        let game = Minesweeper::new(8, 8, 9, &mut rng);
        assert!(game.is_ok());
    }

    #[test]
    fn test_new_error() {
        let mut rng = ThreadRng::default();
        let game = Minesweeper::new(8, 8, 100, &mut rng);
        assert!(game.is_err());
        assert_eq!(game.unwrap_err(), MinesweeperError::TooManyMines);
    }
}#[cfg(test)]
mod tests_llm_16_44 {
    use super::*;

use crate::*;
    use rand::rngs::ThreadRng;

    #[test]
    fn test_randomize() {
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut ThreadRng::default()).unwrap();

        let mut rng = rand::thread_rng();
        minesweeper.randomize(&mut rng).unwrap();

        assert_eq!(minesweeper.board.len(), 64);
    }
}mod tests_llm_16_45 {
    use super::*;

use crate::*;
    use crate::minesweeper::{Minesweeper, Cell, MinesweeperError, GameState};
    use rand::rngs::ThreadRng;
    use rand::Rng;

    #[test]
    fn test_reveal_from_no_adjacent_mine() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 1,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
            ],
            height: 2,
            width: 2,
            status: GameState::InProgress,
        };

        minesweeper.reveal_from(0);

        assert_eq!(minesweeper.board[0].is_revealed, true);
        assert_eq!(minesweeper.board[1].is_revealed, true);
        assert_eq!(minesweeper.board[2].is_revealed, true);
        assert_eq!(minesweeper.board[3].is_revealed, true);
    }

    #[test]
    fn test_reveal_from_adjacent_mine() {
        let mut minesweeper = Minesweeper {
            board: vec![
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 1,
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0,
                },
            ],
            height: 2,
            width: 2,
            status: GameState::InProgress,
        };

        minesweeper.reveal_from(0);

        assert_eq!(minesweeper.board[0].is_revealed, true);
        assert_eq!(minesweeper.board[1].is_revealed, true);
        assert_eq!(minesweeper.board[2].is_revealed, true);
        assert_eq!(minesweeper.board[3].is_revealed, false);
    }
}#[cfg(test)]
mod tests_llm_16_46 {
    use super::*;

use crate::*;
    use rand::rngs::ThreadRng;

    #[test]
    fn test_status() {
        let mut rng = ThreadRng::default();
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();
        let status = minesweeper.status();
        assert_eq!(*status, GameState::InProgress);
    }
}#[cfg(test)]
mod tests_llm_16_47 {
    use super::*;

use crate::*;
    use rand::prelude::*;

    #[test]
    #[should_panic(expected = "Invalid position")]
    fn test_toggle_flag_invalid_position_panic() {
        let mut rng = thread_rng();
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();
        minesweeper.toggle_flag(10, 10).unwrap();
    }

    #[test]
    fn test_toggle_flag_game_ended() {
        let mut rng = thread_rng();
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();
        minesweeper.status = GameState::Win;
        assert_eq!(
            minesweeper.toggle_flag(3, 3).unwrap_err(),
            MinesweeperError::GameEnded
        );
    }

    #[test]
    fn test_toggle_flag_already_revealed() {
        let mut rng = thread_rng();
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();
        minesweeper.board[3 * minesweeper.width + 3].is_revealed = true;
        assert_eq!(
            minesweeper.toggle_flag(3, 3).unwrap_err(),
            MinesweeperError::AlreadyRevealed
        );
    }

    #[test]
    fn test_toggle_flag_flag_cell() {
        let mut rng = thread_rng();
        let mut minesweeper = Minesweeper::new(8, 8, 9, &mut rng).unwrap();
        let flag_state = minesweeper.board[3 * minesweeper.width + 3].is_flagged;
        minesweeper.toggle_flag(3, 3).unwrap();
        assert_eq!(
            minesweeper.board[3 * minesweeper.width + 3].is_flagged,
            !flag_state
        );
    }
}#[cfg(test)]
mod tests_llm_16_48 {
    use super::*;

use crate::*;
    use crate::minesweeper::Minesweeper;
    use rand::rngs::mock::StepRng;

    #[test]
    fn test_update_around_mine_count() {
        let mut minesweeper = Minesweeper {
            height: 3,
            width: 3,
            board: vec![
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: true,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
                Cell {
                    is_mine: false,
                    is_revealed: false,
                    is_flagged: false,
                    mine_adjacent: 0
                },
            ],
            status: GameState::InProgress
        };
        minesweeper.update_around_mine_count();

        assert_eq!(minesweeper.board[0].mine_adjacent, 2);
        assert_eq!(minesweeper.board[1].mine_adjacent, 2);
        assert_eq!(minesweeper.board[2].mine_adjacent, 2);
        assert_eq!(minesweeper.board[3].mine_adjacent, 3);
        assert_eq!(minesweeper.board[4].mine_adjacent, 3);
        assert_eq!(minesweeper.board[5].mine_adjacent, 2);
        assert_eq!(minesweeper.board[6].mine_adjacent, 3);
        assert_eq!(minesweeper.board[7].mine_adjacent, 3);
        assert_eq!(minesweeper.board[8].mine_adjacent, 1);
    }
}