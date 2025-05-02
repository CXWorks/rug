use crate::color::{ColoredString, Colors, Elem};
use std::fs::Metadata;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct INode {
    index: Option<u64>,
}

impl<'a> From<&'a Metadata> for INode {
    #[cfg(unix)]
    fn from(meta: &Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;

        let index = meta.ino();

        Self { index: Some(index) }
    }

    #[cfg(windows)]
    fn from(_: &Metadata) -> Self {
        Self { index: None }
    }
}

impl INode {
    pub fn render(&self, colors: &Colors) -> ColoredString {
        match self.index {
            Some(i) => colors.colorize(i.to_string(), &Elem::INode { valid: true }),
            None => colors.colorize(String::from("-"), &Elem::INode { valid: false }),
        }
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::INode;
    use std::env;
    use std::io;
    use std::path::Path;
    use std::process::{Command, ExitStatus};

    fn cross_platform_touch(path: &Path) -> io::Result<ExitStatus> {
        Command::new("touch").arg(&path).status()
    }

    #[test]
    fn test_inode_no_zero() {
        let mut file_path = env::temp_dir();
        file_path.push("inode.tmp");

        let success = cross_platform_touch(&file_path).unwrap().success();
        assert!(success, "failed to exec touch");

        let inode = INode::from(&file_path.metadata().unwrap());

        #[cfg(unix)]
        assert!(inode.index.is_some());
        #[cfg(windows)]
        assert!(inode.index.is_none());
    }
}
#[cfg(test)]
mod tests_llm_16_121_llm_16_120 {
    use super::*;

use crate::*;
    use crate::meta::inode::INode;
    use std::fs::Metadata;
    use std::os::unix::fs::MetadataExt;

    #[cfg(unix)]
    #[test]
    fn test_from_unix() {
        let meta = Metadata::from(std::fs::metadata("test.txt").unwrap());
        let node: INode = From::from(&meta);
        assert_eq!(node.index, Some(meta.ino()));
    }

    #[cfg(windows)]
    #[test]
    fn test_from_windows() {
        let meta = Metadata::from(std::fs::metadata("test.txt").unwrap());
        let node: INode = From::from(&meta);
        assert_eq!(node.index, None);
    }
}#[cfg(test)]
mod tests_llm_16_253 {
    use super::*;

use crate::*;
    use crate::color::{Colors, ThemeOption};
    use crate::flags::symlink_arrow::SymlinkArrow;
    use crate::meta::inode::INode;

    #[test]
    fn test_render_with_index() {
        let colors = Colors::new(ThemeOption::Default);
        let inode = INode { index: Some(123) };
        let result = inode.render(&colors);
        assert_eq!(result, colors.colorize("123".to_string(), &Elem::INode { valid: true }));
    }
    
    #[test]
    fn test_render_without_index() {
        let colors = Colors::new(ThemeOption::Default);
        let inode = INode { index: None };
        let result = inode.render(&colors);
        assert_eq!(result, colors.colorize("-".to_string(), &Elem::INode { valid: false }));
    }
}