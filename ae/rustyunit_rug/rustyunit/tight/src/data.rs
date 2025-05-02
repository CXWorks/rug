use crate::date::*;
use crate::expense::*;

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
struct DataError(String);

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for DataError {}

#[derive(Serialize, Deserialize)]
pub struct Datafile {
    pub version: u64,
    pub tags: HashSet<String>,
    pub entries: Vec<Expense>,
}

impl Datafile {
    fn new() -> Datafile {
        Datafile {
            version: 1,
            tags: HashSet::new(),
            entries: vec!(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Datafile, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(file);

        let d: Datafile = serde_json::from_reader(reader)?;

        if d.version != 1 {
            return Err(Box::new(DataError("unknown version in datafile!".into())));
        }

        Ok(d)
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.insert(tag);
    }

    pub fn insert(&mut self, expense: Expense) {
        let mut insert_idx = 0;
        for (idx, saved) in self.entries.iter().enumerate() {
            match saved.compare_dates(&expense) {
                std::cmp::Ordering::Greater => { insert_idx = idx; break; },
                std::cmp::Ordering::Less    => { insert_idx = idx + 1; },
                std::cmp::Ordering::Equal   => { insert_idx = idx + 1; },
            }
        }

        if insert_idx > self.entries.len() {
            self.entries.push(expense);
        } else {
            self.entries.insert(insert_idx, expense);
        }
    }

    pub fn remove(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        let mut rm_idx = 0;
        for (idx, saved) in self.entries.iter().enumerate() {
            if saved.compare_id(id) {
                rm_idx = idx;
                break;
            }
        }

        if rm_idx > self.entries.len() {
            return Err(Box::new(DataError("couldn't find item".into())));
        }

        self.entries.remove(rm_idx);
        Ok(())
    }

    pub fn find(&self, id: u64) -> Option<&Expense> {
        for expense in &self.entries {
            if expense.compare_id(id) {
                return Some(expense);
            }
        }

        None
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer(writer, &self)?;

        Ok(())
    }

    // TODO make this faster
    pub fn expenses_between(&self, start: &SimpleDate, end: &SimpleDate) -> &[Expense] {
        let mut start_idx = 0;
        for (idx, expense) in self.entries.iter().enumerate() {
            if let Some(end_date) = expense.get_end_date() {
                if end_date > start {
                    start_idx = idx;
                    break;
                }
            } else {
                start_idx = idx;
                break;
            }
        }

        let mut end_idx = self.entries.len();
        for (idx, expense) in self.entries[start_idx..].iter().enumerate() {
            if expense.get_start_date() > end {
                end_idx = idx + start_idx;
                break;
            }
        }

        &self.entries[start_idx..end_idx]
    }
}

pub fn initialise<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true)
        .create_new(true)
        .open(path)?;
    let contents = serde_json::to_string(&Datafile::new())?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_single() {
        let mut datafile = Datafile::new();
        let expense = Expense::new(0, "test".into(), 100, SimpleDate::from_ymd(2020, 10, 14), None, None, vec!());

        datafile.insert(expense);

        assert_eq!(datafile.entries.len(), 1);
    }

    #[test]
    fn insert_sorted() {
        let mut datafile = Datafile::new();
        let expense1 = Expense::new(0, "test".into(), 100, SimpleDate::from_ymd(2020, 10, 14), None, None, vec!());
        let expense2 = Expense::new(1, "test".into(), 101, SimpleDate::from_ymd(2020, 10, 15), None, None, vec!());

        datafile.insert(expense2);
        datafile.insert(expense1);

        assert_eq!(datafile.entries.len(), 2);
        assert_eq!(datafile.entries[0].amount(), 100);
        assert_eq!(datafile.entries[1].amount(), 101);
    }

    #[test]
    fn remove() {
        let mut datafile = Datafile::new();
        let expense1 = Expense::new(0, "test".into(), 100, SimpleDate::from_ymd(2020, 10, 14), None, None, vec!());
        let expense2 = Expense::new(1, "test".into(), 101, SimpleDate::from_ymd(2020, 10, 15), None, None, vec!());

        datafile.insert(expense2);
        datafile.insert(expense1);

        assert_eq!(datafile.entries.len(), 2);

        assert!(datafile.remove(1).is_ok());

        assert_eq!(datafile.entries.len(), 1);
        assert_eq!(datafile.entries[0].amount(), 100);
    }

    #[test]
    fn find() {
        let mut datafile = Datafile::new();
        let expense1 = Expense::new(0, "test".into(), 100, SimpleDate::from_ymd(2020, 10, 14), None, None, vec!());
        let expense2 = Expense::new(1, "test".into(), 101, SimpleDate::from_ymd(2020, 10, 15), None, None, vec!());

        datafile.insert(expense2);
        datafile.insert(expense1);

        assert!(datafile.find(9999).is_none());

        assert!(datafile.find(1).is_some());
        assert_eq!(datafile.find(1).unwrap().amount(), 101);
    }
}
#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;

use crate::*;
    use std::path::Path;
    use serde_json;

    #[test]
    fn test_add_tag() {
        let mut data = Datafile::new();
        let tag = String::from("test_tag");
        data.add_tag(tag.clone());

        assert!(data.tags.contains(&tag));
    }
}#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;

use crate::*;
    use std::path::Path;

    #[test]
    fn test_from_file() {
        let path: &Path = Path::new("example.json");
        let result = Datafile::from_file(path);

        assert!(result.is_ok());

        let datafile = result.unwrap();

        assert_eq!(datafile.version, 1);

        // Add more assertions as needed
    }
}#[cfg(test)]
mod tests_llm_16_18 {
    use super::*;

use crate::*;
    use std::path::PathBuf;

    #[test]
    fn test_insert() {
        let mut datafile = Datafile::new();
        let expense = Expense::new(1, "Expense 1".into(), 100, SimpleDate::from_ymd(2022, 1, 1), None, None, vec![]);
        datafile.insert(expense);
        assert_eq!(datafile.entries.len(), 1);
    }
}#[cfg(test)]
mod tests_llm_16_19 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let datafile = Datafile::new();
        assert_eq!(datafile.version, 1);
        assert_eq!(datafile.tags.len(), 0);
        assert_eq!(datafile.entries.len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;

use crate::*;
    use std::fs::File;
    use std::io::{Read, Write};

    #[test]
    fn test_initialise() {
        let path = "test_data.json";

        // Call the initialise function
        let result = initialise(path);

        // Assert that the function call succeeded
        assert!(result.is_ok());

        // Read the contents of the file
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        // Assert that the file contains the expected contents
        let expected_contents = serde_json::to_string(&Datafile::new()).unwrap();
        assert_eq!(contents, expected_contents);

        // Clean up the test file
        std::fs::remove_file(path).unwrap();
    }
}#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::data::Datafile;
    
    #[test]
    fn test_remove() {
        let mut p0 = Datafile::new();
        let p1: u64 = 42;
        
        p0.remove(p1).unwrap();
        
        // assert statements here
    }
}#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::data::{Datafile, Expense};

    #[test]
    fn test_rug() {
        let mut p0 = Datafile::new();
        
        // add some expenses to p0.entries
        
        let p1: u64 = 12345;
        
        p0.find(p1);
    }
}#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use std::path::Path;
    use std::io::Write;
    use std::fs::OpenOptions;
    
    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_3_prepare {
            use super::*;
        
            #[test]
            fn sample() {
                let mut v1 = Datafile::new();
            }
        }
        
        let mut p0 = Datafile::new();
        let p1: &Path = Path::new("test.json");
        
        crate::data::Datafile::save(&p0, &p1);
        
        // Verify the file has been created
        
        let file_exists = Path::new("test.json").exists();
        assert_eq!(file_exists, true);
    
        // Clean up the file
        std::fs::remove_file("test.json").unwrap();
    }
}#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::data::Datafile;
    use crate::date::SimpleDate;

    #[test]
    fn test_rug() {
        let mut p0 = Datafile::new();
        let mut p1 = SimpleDate::from_ymd(2021, 9, 14);
        let mut p2 = SimpleDate::from_ymd(2021, 9, 15);

        Datafile::expenses_between(&mut p0, &mut p1, &mut p2);
    }
}