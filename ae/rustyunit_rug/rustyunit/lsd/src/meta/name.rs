use crate::color::{ColoredString, Colors, Elem};
use crate::flags::HyperlinkOption;
use crate::icon::Icons;
use crate::meta::filetype::FileType;
use crate::print_error;
use crate::url::Url;
use std::cmp::{Ordering, PartialOrd};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum DisplayOption<'a> {
    FileName,
    Relative { base_path: &'a Path },
    None,
}

#[derive(Clone, Debug, Eq)]
pub struct Name {
    pub name: String,
    path: PathBuf,
    extension: Option<String>,
    file_type: FileType,
}

impl Name {
    pub fn new(path: &Path, file_type: FileType) -> Self {
        let name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => path.to_string_lossy().to_string(),
        };

        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string());

        Self {
            name,
            path: PathBuf::from(path),
            extension,
            file_type,
        }
    }

    pub fn file_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or(&self.name)
    }

    fn relative_path<T: AsRef<Path> + Clone>(&self, base_path: T) -> PathBuf {
        let base_path = base_path.as_ref();

        if self.path == base_path {
            return PathBuf::from(AsRef::<Path>::as_ref(&Component::CurDir));
        }

        let shared_components: PathBuf = self
            .path
            .components()
            .zip(base_path.components())
            .take_while(|(target_component, base_component)| target_component == base_component)
            .map(|tuple| tuple.0)
            .collect();

        base_path
            .strip_prefix(&shared_components)
            .unwrap()
            .components()
            .map(|_| Component::ParentDir)
            .chain(
                self.path
                    .strip_prefix(&shared_components)
                    .unwrap()
                    .components(),
            )
            .collect()
    }

    pub fn escape(&self, string: &str) -> String {
        if string
            .chars()
            .all(|c| c >= 0x20 as char && c != 0x7f as char)
        {
            string.to_string()
        } else {
            let mut chars = String::new();
            for c in string.chars() {
                // The `escape_default` method on `char` is *almost* what we want here, but
                // it still escapes non-ASCII UTF-8 characters, which are still printable.
                if c >= 0x20 as char && c != 0x7f as char {
                    chars.push(c);
                } else {
                    chars += &c.escape_default().collect::<String>();
                }
            }
            chars
        }
    }

    fn hyperlink(&self, name: String, hyperlink: HyperlinkOption) -> String {
        match hyperlink {
            HyperlinkOption::Always => {
                // HyperlinkOption::Auto gets converted to None or Always in core.rs based on tty_available
                match std::fs::canonicalize(&self.path) {
                    Ok(rp) => {
                        match Url::from_file_path(&rp) {
                            Ok(url) => {
                                // Crossterm does not support hyperlinks as of now
                                // https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
                                format!("\x1B]8;;{}\x1B\x5C{}\x1B]8;;\x1B\x5C", url, name)
                            }
                            Err(_) => {
                                print_error!("{}: unable to form url.", name);
                                name
                            }
                        }
                    }
                    Err(err) => {
                        print_error!("{}: {}.", name, err);
                        name
                    }
                }
            }
            _ => name,
        }
    }

    pub fn render(
        &self,
        colors: &Colors,
        icons: &Icons,
        display_option: &DisplayOption,
        hyperlink: HyperlinkOption,
    ) -> ColoredString {
        let content = match display_option {
            DisplayOption::FileName => {
                format!(
                    "{}{}",
                    icons.get(self),
                    self.hyperlink(self.escape(self.file_name()), hyperlink)
                )
            }
            DisplayOption::Relative { base_path } => format!(
                "{}{}",
                icons.get(self),
                self.hyperlink(
                    self.escape(&self.relative_path(base_path).to_string_lossy()),
                    hyperlink
                )
            ),
            DisplayOption::None => format!(
                "{}{}",
                icons.get(self),
                self.hyperlink(self.escape(&self.path.to_string_lossy()), hyperlink)
            ),
        };

        let elem = match self.file_type {
            FileType::CharDevice => Elem::CharDevice,
            FileType::Directory { uid } => Elem::Dir { uid },
            FileType::SymLink { .. } => Elem::SymLink,
            FileType::File { uid, exec } => Elem::File { uid, exec },
            _ => Elem::File {
                exec: false,
                uid: false,
            },
        };

        colors.colorize_using_path(content, &self.path, &elem)
    }

    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.to_lowercase().cmp(&other.name.to_lowercase())
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name
            .to_lowercase()
            .partial_cmp(&other.name.to_lowercase())
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&other.name.to_lowercase())
    }
}

#[cfg(test)]
mod test {
    use super::DisplayOption;
    use super::Name;
    use crate::color::{self, Colors};
    use crate::flags::HyperlinkOption;
    use crate::icon::{self, Icons};
    use crate::meta::FileType;
    use crate::meta::Meta;
    #[cfg(unix)]
    use crate::meta::Permissions;
    use crate::url::Url;
    use crossterm::style::{Color, Stylize};
    use std::cmp::Ordering;
    use std::fs::{self, File};
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    #[cfg(unix)] // Windows uses different default permissions
    fn test_print_file_name() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the file;
        let file_path = tmp_dir.path().join("file.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = file_path.metadata().expect("failed to get metas");

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, None, &Permissions::from(&meta));
        let name = Name::new(&file_path, file_type);

        assert_eq!(
            " file.txt".to_string().with(Color::AnsiValue(184)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }

    #[test]
    fn test_print_dir_name() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the directory
        let dir_path = tmp_dir.path().join("directory");
        fs::create_dir(&dir_path).expect("failed to create the dir");
        let meta = Meta::from_path(&dir_path, false).unwrap();

        let colors = Colors::new(color::ThemeOption::NoLscolors);

        assert_eq!(
            " directory".to_string().with(Color::AnsiValue(33)),
            meta.name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }

    #[test]
    #[cfg(unix)] // Symlinks are hard on Windows
    fn test_print_symlink_name_file() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the file;
        let file_path = tmp_dir.path().join("file.tmp");
        File::create(&file_path).expect("failed to create file");

        // Create the symlink
        let symlink_path = tmp_dir.path().join("target.tmp");
        symlink(&file_path, &symlink_path).expect("failed to create symlink");
        let meta = symlink_path
            .symlink_metadata()
            .expect("failed to get metas");
        let target_meta = symlink_path.metadata().ok();

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, target_meta.as_ref(), &Permissions::from(&meta));
        let name = Name::new(&symlink_path, file_type);

        assert_eq!(
            " target.tmp".to_string().with(Color::AnsiValue(44)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }

    #[test]
    #[cfg(unix)] // Symlinks are hard on Windows
    fn test_print_symlink_name_dir() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the directory;
        let dir_path = tmp_dir.path().join("tmp.d");
        std::fs::create_dir(&dir_path).expect("failed to create dir");

        // Create the symlink
        let symlink_path = tmp_dir.path().join("target.d");
        symlink(&dir_path, &symlink_path).expect("failed to create symlink");
        let meta = symlink_path
            .symlink_metadata()
            .expect("failed to get metas");
        let target_meta = symlink_path.metadata().ok();

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, target_meta.as_ref(), &Permissions::from(&meta));
        let name = Name::new(&symlink_path, file_type);

        assert_eq!(
            " target.d".to_string().with(Color::AnsiValue(44)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_print_other_type_name() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the pipe;
        let pipe_path = tmp_dir.path().join("pipe.tmp");
        let success = Command::new("mkfifo")
            .arg(&pipe_path)
            .status()
            .expect("failed to exec mkfifo")
            .success();
        assert_eq!(true, success, "failed to exec mkfifo");
        let meta = pipe_path.metadata().expect("failed to get metas");

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, None, &Permissions::from(&meta));
        let name = Name::new(&pipe_path, file_type);

        assert_eq!(
            " pipe.tmp".to_string().with(Color::AnsiValue(184)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }

    #[test]
    fn test_print_without_icon_or_color() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::NoIcon, " ".to_string());

        // Create the file;
        let file_path = tmp_dir.path().join("file.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = Meta::from_path(&file_path, false).unwrap();

        let colors = Colors::new(color::ThemeOption::NoColor);

        assert_eq!(
            "file.txt",
            meta.name
                .render(
                    &colors,
                    &icons,
                    &DisplayOption::FileName,
                    HyperlinkOption::Never
                )
                .to_string()
                .as_str()
        );
    }

    #[test]
    fn test_print_hyperlink() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::NoIcon, " ".to_string());

        // Create the file;
        let file_path = tmp_dir.path().join("file.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = Meta::from_path(&file_path, false).unwrap();

        let colors = Colors::new(color::ThemeOption::NoColor);

        let real_path = std::fs::canonicalize(&file_path).expect("canonicalize");
        let expected_url = Url::from_file_path(&real_path).expect("absolute path");
        let expected_text = format!(
            "\x1B]8;;{}\x1B\x5C{}\x1B]8;;\x1B\x5C",
            expected_url, "file.txt"
        );

        assert_eq!(
            expected_text,
            meta.name
                .render(
                    &colors,
                    &icons,
                    &DisplayOption::FileName,
                    HyperlinkOption::Always
                )
                .to_string()
                .as_str()
        );
    }

    #[test]
    fn test_extensions_with_valid_file() {
        let path = Path::new("some-file.txt");

        let name = Name::new(
            &path,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(Some("txt"), name.extension());
    }

    #[test]
    fn test_extensions_with_file_without_extension() {
        let path = Path::new(".gitignore");

        let name = Name::new(
            &path,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(None, name.extension());
    }

    #[test]
    fn test_order_impl_is_case_insensitive() {
        let path_1 = Path::new("/AAAA");
        let name_1 = Name::new(
            &path_1,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        let path_2 = Path::new("/aaaa");
        let name_2 = Name::new(
            &path_2,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(Ordering::Equal, name_1.cmp(&name_2));
    }

    #[test]
    fn test_partial_order_impl() {
        let path_a = Path::new("/aaaa");
        let name_a = Name::new(
            &path_a,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        let path_z = Path::new("/zzzz");
        let name_z = Name::new(
            &path_z,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(true, name_a < name_z);
    }

    #[test]
    fn test_partial_order_impl_is_case_insensitive() {
        let path_a = Path::new("aaaa");
        let name_a = Name::new(
            &path_a,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        let path_z = Path::new("ZZZZ");
        let name_z = Name::new(
            &path_z,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(true, name_a < name_z);
    }

    #[test]
    fn test_partial_eq_impl() {
        let path_1 = Path::new("aaaa");
        let name_1 = Name::new(
            &path_1,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        let path_2 = Path::new("aaaa");
        let name_2 = Name::new(
            &path_2,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(true, name_1 == name_2);
    }

    #[test]
    fn test_partial_eq_impl_is_case_insensitive() {
        let path_1 = Path::new("AAAA");
        let name_1 = Name::new(
            &path_1,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        let path_2 = Path::new("aaaa");
        let name_2 = Name::new(
            &path_2,
            FileType::File {
                uid: false,
                exec: false,
            },
        );

        assert_eq!(true, name_1 == name_2);
    }

    #[test]
    fn test_parent_relative_path() {
        let name = Name::new(
            Path::new("/home/parent1/child"),
            FileType::File {
                uid: false,
                exec: false,
            },
        );
        let base_path = Path::new("/home/parent2");

        assert_eq!(
            PathBuf::from("../parent1/child"),
            name.relative_path(base_path),
        )
    }

    #[test]
    fn test_current_relative_path() {
        let name = Name::new(
            Path::new("/home/parent1/child"),
            FileType::File {
                uid: false,
                exec: false,
            },
        );
        let base_path = PathBuf::from("/home/parent1");

        assert_eq!(PathBuf::from("child"), name.relative_path(base_path),)
    }

    #[test]
    fn test_grand_parent_relative_path() {
        let name = Name::new(
            Path::new("/home/grand-parent1/parent1/child"),
            FileType::File {
                uid: false,
                exec: false,
            },
        );
        let base_path = PathBuf::from("/home/grand-parent2/parent1");

        assert_eq!(
            PathBuf::from("../../grand-parent1/parent1/child"),
            name.relative_path(base_path),
        )
    }

    #[test]
    #[cfg(unix)]
    fn test_special_chars_in_filename() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let icons = Icons::new(icon::Theme::Fancy, " ".to_string());

        // Create the file;
        let file_path = tmp_dir.path().join("file\ttab.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = file_path.metadata().expect("failed to get metas");

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, None, &Permissions::from(&meta));
        let name = Name::new(&file_path, file_type);

        assert_eq!(
            " file\\ttab.txt".to_string().with(Color::AnsiValue(184)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );

        let file_path = tmp_dir.path().join("file\nnewline.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = file_path.metadata().expect("failed to get metas");

        let colors = Colors::new(color::ThemeOption::NoLscolors);
        let file_type = FileType::new(&meta, None, &Permissions::from(&meta));
        let name = Name::new(&file_path, file_type);

        assert_eq!(
            " file\\nnewline.txt"
                .to_string()
                .with(Color::AnsiValue(184)),
            name.render(
                &colors,
                &icons,
                &DisplayOption::FileName,
                HyperlinkOption::Never
            )
        );
    }
}
#[cfg(test)]
mod tests_llm_16_124 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;

    #[test]
    fn test_cmp() {
        let name1 = Name {
            name: String::from("abc"),
            path: PathBuf::from(""),
            extension: None,
            file_type: FileType::File {
                uid: false,
                exec: false,
            },
        };
        let name2 = Name {
            name: String::from("def"),
            path: PathBuf::from(""),
            extension: None,
            file_type: FileType::File {
                uid: false,
                exec: false,
            },
        };
        let result = name1.cmp(&name2);
        assert_eq!(result, Ordering::Less);
    }
}#[cfg(test)]
mod tests_llm_16_125 {
    use super::*;

use crate::*;
    use std::path::Path;
    use crate::meta::filetype::FileType;
    use crate::meta::filetype::FileType::*;
    
    #[test]
    fn test_eq() {
        let name1 = Name {
            name: String::from("test.txt"),
            path: Path::new("path/to/test.txt").to_path_buf(),
            extension: Some(String::from("txt")),
            file_type: File { uid: false, exec: false },
        };

        let name2 = Name {
            name: String::from("TEST.txt"),
            path: Path::new("path/to/TEST.txt").to_path_buf(),
            extension: Some(String::from("txt")),
            file_type: File { uid: false, exec: false },
        };

        let name3 = Name {
            name: String::from("test.txt"),
            path: Path::new("path/to/other.txt").to_path_buf(),
            extension: Some(String::from("txt")),
            file_type: File { uid: false, exec: false },
        };

        let name4 = Name {
            name: String::from("test.txt"),
            path: Path::new("path/to/test.txt").to_path_buf(),
            extension: Some(String::from("pdf")),
            file_type: File { uid: false, exec: false },
        };

        let name5 = Name {
            name: String::from("test.txt"),
            path: Path::new("path/to/test.txt").to_path_buf(),
            extension: Some(String::from("txt")),
            file_type: File { uid: true, exec: false },
        };

        let name6 = Name {
            name: String::from("test.txt"),
            path: Path::new("path/to/test.txt").to_path_buf(),
            extension: Some(String::from("txt")),
            file_type: File { uid: false, exec: true },
        };

        assert_eq!(name1.eq(&name2), true);
        assert_eq!(name1.eq(&name3), false);
        assert_eq!(name1.eq(&name4), false);
        assert_eq!(name1.eq(&name5), true);
        assert_eq!(name1.eq(&name6), true);
    }
}#[cfg(test)]
mod tests_llm_16_126 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_partial_cmp() {
        let name1 = Name {
            name: "file1".to_string(),
            path: PathBuf::from(""),
            extension: None,
            file_type: FileType::File {
                uid: false,
                exec: false
            }
        };
        let name2 = Name {
            name: "file2".to_string(),
            path: PathBuf::from(""),
            extension: None,
            file_type: FileType::File {
                uid: false,
                exec: false
            }
        };
        let name3 = Name {
            name: "file1".to_string(),
            path: PathBuf::from(""),
            extension: None,
            file_type: FileType::File {
                uid: false,
                exec: false
            }
        };
        
        let result1 = name1.partial_cmp(&name2);
        let result2 = name1.partial_cmp(&name3);
        
        assert_eq!(result1, Some(Ordering::Less));
        assert_eq!(result2, Some(Ordering::Equal));
    }
}#[cfg(test)]
mod tests_llm_16_256 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_escape_all_printable_chars() {
        let name = Name {
            name: String::from("test"),
            path: PathBuf::new(),
            extension: None,
            file_type: FileType::File {
                exec: false,
                uid: false,
            },
        };
        let actual = name.escape("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ");
        let expected = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string();
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn test_escape_non_printable_chars() {
        let name = Name {
            name: String::from("test"),
            path: PathBuf::new(),
            extension: None,
            file_type: FileType::File {
                exec: false,
                uid: false,
            },
        };
        let actual = name.escape("Hello\tWorld");
        let expected = "Hello\\tWorld".to_string();
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn test_escape_mixed_chars() {
        let name = Name {
            name: String::from("test"),
            path: PathBuf::new(),
            extension: None,
            file_type: FileType::File {
                exec: false,
                uid: false,
            },
        };
        let actual = name.escape("Hello\tWorld!@#$%^&*()_+1234567890-=");
        let expected = "Hello\\tWorld\\!\\@\\#\\$\\%\\^\\&\\*\\(\\)\\_\\+1234567890\\-=".to_string();
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn test_escape_non_ascii_chars() {
        let name = Name {
            name: String::from("test"),
            path: PathBuf::new(),
            extension: None,
            file_type: FileType::File {
                exec: false,
                uid: false,
            },
        };
        let actual = name.escape("你好，世界！");
        let expected = "\\u4f60\\u597d\\uFF0C\\u4E16\\u754C\\uFF01".to_string();
        assert_eq!(actual, expected);
    }
}#[cfg(test)]
mod tests_llm_16_257 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_extension() {
        let name = Name {
            name: String::from("example.txt"),
            path: PathBuf::from("path/to/example.txt"),
            extension: Some(String::from("txt")),
            file_type: FileType::File {
                uid: false,
                exec: false,
            },
        };
        assert_eq!(name.extension(), Some("txt"));

        let name = Name {
            name: String::from("example"),
            path: PathBuf::from("path/to/example"),
            extension: None,
            file_type: FileType::Directory { uid: false },
        };
        assert_eq!(name.extension(), None);
    }
}#[cfg(test)]
mod tests_llm_16_258 {
    use super::*;

use crate::*;

    #[test]
    fn test_file_name() {
        let name = Name {
            name: String::from("example.txt"),
            path: PathBuf::from("path/to/example.txt"),
            extension: Some(String::from("txt")),
            file_type: FileType::File {
                uid: false,
                exec: false,
            },
        };

        assert_eq!(name.file_name(), "example.txt");
    }
}#[cfg(test)]
mod tests_llm_16_261 {
    use super::*;

use crate::*;
    use crate::flags::hyperlink::HyperlinkOption;

    #[test]
    fn test_hyperlink() {
        let name = Name {
            name: "test.txt".to_string(),
            path: PathBuf::from("/path/to/test.txt"),
            extension: Some("txt".to_string()),
            file_type: FileType::File {
                uid: false,
                exec: false,
            },
        };

        let result = name.hyperlink("test.txt".to_string(), HyperlinkOption::Always);

        assert_eq!(result, "\x1B]8;;file:///path/to/test.txt\x1B\x5Ctest.txt\x1B]8;;\x1B\x5C");
    }
}#[cfg(test)]
mod tests_llm_16_262 {
    use super::*;

use crate::*;
    use std::path::Path;

    #[test]
    fn test_new() {
        let path = Path::new("test-file");
        let file_type = FileType::File { uid: false, exec: false };
        let name = Name::new(&path, file_type);

        assert_eq!(name.name, "test-file");
        assert_eq!(name.path, PathBuf::from("test-file"));
        assert_eq!(name.extension, None);
        assert_eq!(name.file_type, FileType::File { uid: false, exec: false });
    }
}#[cfg(test)]
mod tests_llm_16_263 {
    use super::*;

use crate::*;
    use std::path::Path;

    #[test]
    fn test_relative_path() {
        let name = Name {
            name: "test.txt".to_string(),
            path: PathBuf::from("/path/to/test.txt"),
            extension: Some("txt".to_string()),
            file_type: FileType::File { uid: false, exec: false },
        };

        let base_path = "/path/to";

        let result = name.relative_path(base_path);

        let expected = PathBuf::from("test.txt");

        assert_eq!(result, expected);
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use std::path::Path;
    use crate::meta::Name;
    use crate::meta::FileType;

    #[test]
    fn test_rug() {
        let path = Path::new("/path/to/file.txt");
        let file_type = FileType::File { uid: false, exec: false };
        let p0 = Name::new(path, file_type);
                
        p0.file_type();

    }
}
