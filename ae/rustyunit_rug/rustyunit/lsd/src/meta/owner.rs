use crate::color::{ColoredString, Colors, Elem};
#[cfg(unix)]
use std::fs::Metadata;

#[derive(Clone, Debug)]
pub struct Owner {
    user: String,
    group: String,
}

impl Owner {
    #[cfg_attr(unix, allow(dead_code))]
    pub fn new(user: String, group: String) -> Self {
        Self { user, group }
    }
}

#[cfg(unix)]
impl<'a> From<&'a Metadata> for Owner {
    fn from(meta: &Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        use users::{get_group_by_gid, get_user_by_uid};

        let user = match get_user_by_uid(meta.uid()) {
            Some(res) => res.name().to_string_lossy().to_string(),
            None => meta.uid().to_string(),
        };

        let group = match get_group_by_gid(meta.gid()) {
            Some(res) => res.name().to_string_lossy().to_string(),
            None => meta.gid().to_string(),
        };

        Self { user, group }
    }
}

impl Owner {
    pub fn render_user(&self, colors: &Colors) -> ColoredString {
        colors.colorize(self.user.clone(), &Elem::User)
    }

    pub fn render_group(&self, colors: &Colors) -> ColoredString {
        colors.colorize(self.group.clone(), &Elem::Group)
    }
}
#[cfg(test)]
mod tests_llm_16_266 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let user = String::from("test_user");
        let group = String::from("test_group");
        let owner = Owner::new(user.clone(), group.clone());

        assert_eq!(owner.user, user);
        assert_eq!(owner.group, group);
    }
}#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::color::{Colors, ThemeOption};
    use crate::meta::owner::Owner;

    #[test]
    fn test_rug() {
        // Construct the first argument
        let mut p0 = Owner::new("user".to_string(), "group".to_string());

        // Construct the second argument
        let mut p1 = Colors::new(ThemeOption::Default);

        crate::meta::owner::Owner::render_user(&p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::meta::owner::Owner;
    use crate::color::{Colors, ThemeOption, Elem, ColoredString};

    #[test]
    fn test_render_group() {
        let mut p0 = Owner::new("user".to_string(), "group".to_string());
        let mut p1 = Colors::new(ThemeOption::Default);

        Owner::render_group(&p0, &p1);
    }
}