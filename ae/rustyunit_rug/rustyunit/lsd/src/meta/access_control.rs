use crate::color::{ColoredString, Colors, Elem};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct AccessControl {
    has_acl: bool,
    selinux_context: String,
    smack_context: String,
}

impl AccessControl {
    #[cfg(not(unix))]
    pub fn for_path(_: &Path) -> Self {
        Self::from_data(false, &[], &[])
    }

    #[cfg(unix)]
    pub fn for_path(path: &Path) -> Self {
        let has_acl = !xattr::get(path, Method::Acl.name())
            .unwrap_or_default()
            .unwrap_or_default()
            .is_empty();
        let selinux_context = xattr::get(path, Method::Selinux.name())
            .unwrap_or_default()
            .unwrap_or_default();
        let smack_context = xattr::get(path, Method::Smack.name())
            .unwrap_or_default()
            .unwrap_or_default();

        Self::from_data(has_acl, &selinux_context, &smack_context)
    }

    fn from_data(has_acl: bool, selinux_context: &[u8], smack_context: &[u8]) -> Self {
        let selinux_context = String::from_utf8_lossy(selinux_context).to_string();
        let smack_context = String::from_utf8_lossy(smack_context).to_string();
        Self {
            has_acl,
            selinux_context,
            smack_context,
        }
    }

    pub fn render_method(&self, colors: &Colors) -> ColoredString {
        if self.has_acl {
            colors.colorize(String::from("+"), &Elem::Acl)
        } else if !self.selinux_context.is_empty() || !self.smack_context.is_empty() {
            colors.colorize(String::from("."), &Elem::Context)
        } else {
            colors.colorize(String::from(""), &Elem::Acl)
        }
    }

    pub fn render_context(&self, colors: &Colors) -> ColoredString {
        let mut context = self.selinux_context.clone();
        if !self.smack_context.is_empty() {
            if !context.is_empty() {
                context += "+";
            }
            context += &self.smack_context;
        }
        if context.is_empty() {
            context += "?";
        }
        colors.colorize(context, &Elem::Context)
    }
}

#[cfg(unix)]
enum Method {
    Acl,
    Selinux,
    Smack,
}

#[cfg(unix)]
impl Method {
    fn name(&self) -> &'static str {
        match self {
            Method::Acl => "system.posix_acl_access",
            Method::Selinux => "security.selinux",
            Method::Smack => "security.SMACK64",
        }
    }
}

#[cfg(test)]
mod test {
    use super::AccessControl;
    use crate::color::{Colors, ThemeOption};
    use crossterm::style::{Color, Stylize};

    #[test]
    fn test_acl_only_indicator() {
        // actual file would collide with proper AC data, no permission to scrub those
        let access_control = AccessControl::from_data(true, &[], &[]);

        assert_eq!(
            String::from("+").with(Color::DarkCyan),
            access_control.render_method(&Colors::new(ThemeOption::Default))
        );
    }

    #[test]
    fn test_smack_only_indicator() {
        let access_control = AccessControl::from_data(false, &[], &[b'a']);

        assert_eq!(
            String::from(".").with(Color::Cyan),
            access_control.render_method(&Colors::new(ThemeOption::Default))
        );
    }

    #[test]
    fn test_acl_and_selinux_indicator() {
        let access_control = AccessControl::from_data(true, &[b'a'], &[]);

        assert_eq!(
            String::from("+").with(Color::DarkCyan),
            access_control.render_method(&Colors::new(ThemeOption::Default))
        );
    }

    #[test]
    fn test_selinux_context() {
        let access_control = AccessControl::from_data(false, &[b'a'], &[]);

        assert_eq!(
            String::from("a").with(Color::Cyan),
            access_control.render_context(&Colors::new(ThemeOption::Default))
        );
    }

    #[test]
    fn test_selinux_and_smack_context() {
        let access_control = AccessControl::from_data(false, &[b'a'], &[b'b']);

        assert_eq!(
            String::from("a+b").with(Color::Cyan),
            access_control.render_context(&Colors::new(ThemeOption::Default))
        );
    }

    #[test]
    fn test_no_context() {
        let access_control = AccessControl::from_data(false, &[], &[]);

        assert_eq!(
            String::from("?").with(Color::Cyan),
            access_control.render_context(&Colors::new(ThemeOption::Default))
        );
    }
}
#[cfg(test)]
mod tests_llm_16_235 {
    use super::*;

use crate::*;
    use std::path::Path;
    
    #[test]
    #[cfg(unix)]
    fn test_for_path_unix() {
        let path = Path::new("/path/to/file");
        let result = AccessControl::for_path(&path);
        // Assert the result here
    }
    
    #[test]
    #[cfg(not(unix))]
    fn test_for_path_non_unix() {
        let path = Path::new("/path/to/file");
        let result = AccessControl::for_path(&path);
        // Assert the result here
    }
}#[cfg(test)]
mod tests_llm_16_236 {
    use super::*;

use crate::*;
    use std::path::Path;
    
    #[test]
    #[cfg(not(unix))]
    fn test_from_data() {
        let has_acl = false;
        let selinux_context = [];
        let smack_context = [];
        let access_control = AccessControl::from_data(has_acl, &selinux_context, &smack_context);

        assert_eq!(access_control.has_acl, false);
        assert_eq!(access_control.selinux_context, "");
        assert_eq!(access_control.smack_context, "");
    }

    #[test]
    #[cfg(unix)]
    fn test_from_data() {
        let path = Path::new("test.txt");
        let access_control = AccessControl::for_path(&path);

        assert_eq!(access_control.has_acl, false);
        assert_eq!(access_control.selinux_context, "");
        assert_eq!(access_control.smack_context, "");
    }
}#[cfg(test)]
mod tests_llm_16_242_llm_16_241 {
    #[cfg(unix)]
    #[test]
    fn test_name() {
        use crate::meta::access_control::Method;

        let acl = Method::Acl;
        let selinux = Method::Selinux;
        let smack = Method::Smack;

        assert_eq!(acl.name(), "system.posix_acl_access");
        assert_eq!(selinux.name(), "security.selinux");
        assert_eq!(smack.name(), "security.SMACK64");
    }
}#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use std::path::Path;
    use crate::color;
    use crate::color::{Colors, ColoredString, Elem};

    #[test]
    fn test_rug() {
        let mut p0 = AccessControl::for_path(Path::new("/path/to/file"));
        let mut p1 = Colors::new(color::ThemeOption::Default);

        p0.render_method(&p1);
    }
}                
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use std::path::Path;
    use crate::color;

    #[test]
    fn test_render_context() {
        let mut p0 = AccessControl::for_path(Path::new("/path/to/file"));
        let mut p1 = color::Colors::new(color::ThemeOption::Default);

        p0.render_context(&p1);
    }
}