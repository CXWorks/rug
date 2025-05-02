use crate::meta::{FileType, Name};
use std::collections::HashMap;

pub struct Icons {
    display_icons: bool,
    icons_by_name: HashMap<&'static str, &'static str>,
    icons_by_extension: HashMap<&'static str, &'static str>,
    default_folder_icon: &'static str,
    default_file_icon: &'static str,
    icon_separator: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Theme {
    NoIcon,
    Fancy,
    Unicode,
}

// In order to add a new icon, write the unicode value like "\ue5fb" then
// run the command below in vim:
//
// s#\\u[0-9a-f]*#\=eval('"'.submatch(0).'"')#
impl Icons {
    pub fn new(theme: Theme, icon_separator: String) -> Self {
        let display_icons = theme == Theme::Fancy || theme == Theme::Unicode;
        let (icons_by_name, icons_by_extension, default_file_icon, default_folder_icon) =
            if theme == Theme::Fancy {
                (
                    Self::get_default_icons_by_name(),
                    Self::get_default_icons_by_extension(),
                    "\u{f016}", // 
                    "\u{f115}", // 
                )
            } else {
                (
                    HashMap::new(),
                    HashMap::new(),
                    "\u{1f5cb}", // 🗋
                    "\u{1f5c1}", // 🗁
                )
            };

        Self {
            display_icons,
            icons_by_name,
            icons_by_extension,
            default_file_icon,
            default_folder_icon,
            icon_separator,
        }
    }

    pub fn get(&self, name: &Name) -> String {
        if !self.display_icons {
            return String::new();
        }

        // Check file types
        let file_type: FileType = name.file_type();

        let icon = if let FileType::Directory { .. } = file_type {
            self.default_folder_icon
        } else if let FileType::SymLink { is_dir: true } = file_type {
            "\u{f482}" // ""
        } else if let FileType::SymLink { is_dir: false } = file_type {
            "\u{f481}" // ""
        } else if let FileType::Socket = file_type {
            "\u{f6a7}" // ""
        } else if let FileType::Pipe = file_type {
            "\u{f731}" // ""
        } else if let FileType::CharDevice = file_type {
            "\u{e601}" // ""
        } else if let FileType::BlockDevice = file_type {
            "\u{fc29}" // "ﰩ"
        } else if let FileType::Special = file_type {
            "\u{f2dc}" // ""
        } else if let Some(icon) = self
            .icons_by_name
            .get(name.file_name().to_lowercase().as_str())
        {
            // Use the known names.
            icon
        } else if let Some(icon) = name.extension().and_then(|extension| {
            self.icons_by_extension
                .get(extension.to_lowercase().as_str())
        }) {
            // Use the known extensions.
            icon
        } else {
            // Use the default icons.
            self.default_file_icon
        };

        format!("{}{}", icon, self.icon_separator)
    }

    fn get_default_icons_by_name() -> HashMap<&'static str, &'static str> {
        let mut m = HashMap::new();

        // Note: filenames must be lower-case

        m.insert(".trash", "\u{f1f8}"); // ""
        m.insert(".atom", "\u{e764}"); // ""
        m.insert(".bashprofile", "\u{e615}"); // ""
        m.insert(".bashrc", "\u{f489}"); // ""
        m.insert(".clang-format", "\u{e615}"); // ""
        m.insert(".git", "\u{f1d3}"); // ""
        m.insert(".gitattributes", "\u{f1d3}"); // ""
        m.insert(".gitconfig", "\u{f1d3}"); // ""
        m.insert(".github", "\u{f408}"); // ""
        m.insert(".gitignore", "\u{f1d3}"); // ""
        m.insert(".gitmodules", "\u{f1d3}"); // ""
        m.insert(".rvm", "\u{e21e}"); // ""
        m.insert(".vimrc", "\u{e62b}"); // ""
        m.insert(".vscode", "\u{e70c}"); // ""
        m.insert(".zshrc", "\u{f489}"); // ""
        m.insert("bin", "\u{e5fc}"); // ""
        m.insert("config", "\u{e5fc}"); // ""
        m.insert("docker-compose.yml", "\u{f308}"); // ""
        m.insert("dockerfile", "\u{f308}"); // ""
        m.insert("ds_store", "\u{f179}"); // ""
        m.insert("gitignore_global", "\u{f1d3}"); // ""
        m.insert("gradle", "\u{e70e}"); // ""
        m.insert("gruntfile.coffee", "\u{e611}"); // ""
        m.insert("gruntfile.js", "\u{e611}"); // ""
        m.insert("gruntfile.ls", "\u{e611}"); // ""
        m.insert("gulpfile.coffee", "\u{e610}"); // ""
        m.insert("gulpfile.js", "\u{e610}"); // ""
        m.insert("gulpfile.ls", "\u{e610}"); // ""
        m.insert("hidden", "\u{f023}"); // ""
        m.insert("include", "\u{e5fc}"); // ""
        m.insert("lib", "\u{f121}"); // ""
        m.insert("localized", "\u{f179}"); // ""
        m.insert("node_modules", "\u{e718}"); // ""
        m.insert("npmignore", "\u{e71e}"); // ""
        m.insert("rubydoc", "\u{e73b}"); // ""

        m
    }

    fn get_default_icons_by_extension() -> HashMap<&'static str, &'static str> {
        let mut m = HashMap::new();

        // Note: extensions must be lower-case

        m.insert("7z", "\u{f410}"); // ""
        m.insert("ai", "\u{e7b4}"); // ""
        m.insert("apk", "\u{e70e}"); // ""
        m.insert("avi", "\u{f03d}"); // ""
        m.insert("avro", "\u{e60b}"); // ""
        m.insert("awk", "\u{f489}"); // ""
        m.insert("bash", "\u{f489}"); // ""
        m.insert("bash_history", "\u{f489}"); // ""
        m.insert("bash_profile", "\u{f489}"); // ""
        m.insert("bashrc", "\u{f489}"); // ""
        m.insert("bat", "\u{f17a}"); // ""
        m.insert("bio", "\u{f910}"); // "蘿"
        m.insert("bmp", "\u{f1c5}"); // ""
        m.insert("bz2", "\u{f410}"); // ""
        m.insert("c", "\u{e61e}"); // ""
        m.insert("c++", "\u{e61d}"); // ""
        m.insert("cc", "\u{e61d}"); // ""
        m.insert("cfg", "\u{e615}"); // ""
        m.insert("clj", "\u{e768}"); // ""
        m.insert("cljs", "\u{e76a}"); // ""
        m.insert("cls", "\u{e600}"); // ""
        m.insert("coffee", "\u{f0f4}"); // ""
        m.insert("conf", "\u{e615}"); // ""
        m.insert("cp", "\u{e61d}"); // ""
        m.insert("cpp", "\u{e61d}"); // ""
        m.insert("cs", "\u{f81a}"); // ""
        m.insert("cshtml", "\u{f1fa}"); // ""
        m.insert("csproj", "\u{f81a}"); // ""
        m.insert("csx", "\u{f81a}"); // ""
        m.insert("csh", "\u{f489}"); // ""
        m.insert("css", "\u{e749}"); // ""
        m.insert("csv", "\u{f1c3}"); // ""
        m.insert("cxx", "\u{e61d}"); // ""
        m.insert("d", "\u{e7af}"); // ""
        m.insert("dart", "\u{e798}"); // ""
        m.insert("db", "\u{f1c0}"); // ""
        m.insert("diff", "\u{f440}"); // ""
        m.insert("doc", "\u{f1c2}"); // ""
        m.insert("dockerfile", "\u{f308}"); // ""
        m.insert("docx", "\u{f1c2}"); // ""
        m.insert("ds_store", "\u{f179}"); // ""
        m.insert("dump", "\u{f1c0}"); // ""
        m.insert("ebook", "\u{e28b}"); // ""
        m.insert("editorconfig", "\u{e615}"); // ""
        m.insert("ejs", "\u{e618}"); // ""
        m.insert("elm", "\u{e62c}"); // ""
        m.insert("env", "\u{f462}"); // ""
        m.insert("eot", "\u{f031}"); // ""
        m.insert("epub", "\u{e28a}"); // ""
        m.insert("erb", "\u{e73b}"); // ""
        m.insert("erl", "\u{e7b1}"); // ""
        m.insert("exe", "\u{f17a}"); // ""
        m.insert("ex", "\u{e62d}"); // ""
        m.insert("exs", "\u{e62d}"); // ""
        m.insert("fish", "\u{f489}"); // ""
        m.insert("flac", "\u{f001}"); // ""
        m.insert("flv", "\u{f03d}"); // ""
        m.insert("font", "\u{f031}"); // ""
        m.insert("fpl", "\u{f910}"); // "蘿"
        m.insert("fs", "\u{e7a7}"); // ""
        m.insert("fsx", "\u{e7a7}"); // ""
        m.insert("fsi", "\u{e7a7}"); // ""
        m.insert("gdoc", "\u{f1c2}"); // ""
        m.insert("gemfile", "\u{e21e}"); // ""
        m.insert("gemspec", "\u{e21e}"); // ""
        m.insert("gform", "\u{f298}"); // ""
        m.insert("gif", "\u{f1c5}"); // ""
        m.insert("git", "\u{f1d3}"); // ""
        m.insert("go", "\u{e626}"); // ""
        m.insert("gradle", "\u{e70e}"); // ""
        m.insert("gsheet", "\u{f1c3}"); // ""
        m.insert("gslides", "\u{f1c4}"); // ""
        m.insert("guardfile", "\u{e21e}"); // ""
        m.insert("gz", "\u{f410}"); // ""
        m.insert("h", "\u{f0fd}"); // ""
        m.insert("hbs", "\u{e60f}"); // ""
        m.insert("heic", "\u{f1c5}"); // ""
        m.insert("heif", "\u{f1c5}"); // ""
        m.insert("heix", "\u{f1c5}"); // ""
        m.insert("hpp", "\u{f0fd}"); // ""
        m.insert("hs", "\u{e777}"); // ""
        m.insert("htm", "\u{f13b}"); // ""
        m.insert("html", "\u{f13b}"); // ""
        m.insert("hxx", "\u{f0fd}"); // ""
        m.insert("ico", "\u{f1c5}"); // ""
        m.insert("image", "\u{f1c5}"); // ""
        m.insert("iml", "\u{e7b5}"); // ""
        m.insert("ini", "\u{e615}"); // ""
        m.insert("ipynb", "\u{e606}"); // ""
        m.insert("jar", "\u{e204}"); // ""
        m.insert("java", "\u{e204}"); // ""
        m.insert("jpeg", "\u{f1c5}"); // ""
        m.insert("jpg", "\u{f1c5}"); // ""
        m.insert("js", "\u{e74e}"); // ""
        m.insert("json", "\u{e60b}"); // ""
        m.insert("jsx", "\u{e7ba}"); // ""
        m.insert("jl", "\u{e624}"); // ""
        m.insert("ksh", "\u{f489}"); // ""
        m.insert("less", "\u{e758}"); // ""
        m.insert("lhs", "\u{e777}"); // ""
        m.insert("license", "\u{f48a}"); // ""
        m.insert("localized", "\u{f179}"); // ""
        m.insert("lock", "\u{f023}"); // ""
        m.insert("log", "\u{f18d}"); // ""
        m.insert("lua", "\u{e620}"); // ""
        m.insert("lz", "\u{f410}"); // ""
        m.insert("m3u", "\u{f910}"); // "蘿"
        m.insert("m3u8", "\u{f910}"); // "蘿"
        m.insert("m4a", "\u{f001}"); // ""
        m.insert("magnet", "\u{f076}"); // ""
        m.insert("markdown", "\u{f48a}"); // ""
        m.insert("md", "\u{f48a}"); // ""
        m.insert("mjs", "\u{e74e}"); // ""
        m.insert("mkd", "\u{f48a}"); // ""
        m.insert("mkv", "\u{f03d}"); // ""
        m.insert("mobi", "\u{e28b}"); // ""
        m.insert("mov", "\u{f03d}"); // ""
        m.insert("mp3", "\u{f001}"); // ""
        m.insert("mp4", "\u{f03d}"); // ""
        m.insert("mustache", "\u{e60f}"); // ""
        m.insert("nix", "\u{f313}"); // ""
        m.insert("npmignore", "\u{e71e}"); // ""
        m.insert("opus", "\u{f001}"); // ""
        m.insert("ogg", "\u{f001}"); // ""
        m.insert("ogv", "\u{f03d}"); // ""
        m.insert("otf", "\u{f031}"); // ""
        m.insert("pdf", "\u{f1c1}"); // ""
        m.insert("pem", "\u{f805}"); // ""
        m.insert("php", "\u{e73d}"); // ""
        m.insert("pl", "\u{e769}"); // ""
        m.insert("pls", "\u{f910}"); // "蘿"
        m.insert("pm", "\u{e769}"); // ""
        m.insert("png", "\u{f1c5}"); // ""
        m.insert("ppt", "\u{f1c4}"); // ""
        m.insert("pptx", "\u{f1c4}"); // ""
        m.insert("procfile", "\u{e21e}"); // ""
        m.insert("properties", "\u{e60b}"); // ""
        m.insert("ps1", "\u{f489}"); // ""
        m.insert("psd", "\u{e7b8}"); // ""
        m.insert("pxm", "\u{f1c5}"); // ""
        m.insert("py", "\u{e606}"); // ""
        m.insert("pyc", "\u{e606}"); // ""
        m.insert("r", "\u{f25d}"); // ""
        m.insert("rakefile", "\u{e21e}"); // ""
        m.insert("rar", "\u{f410}"); // ""
        m.insert("razor", "\u{f1fa}"); // ""
        m.insert("rb", "\u{e21e}"); // ""
        m.insert("rdata", "\u{f25d}"); // ""
        m.insert("rdb", "\u{e76d}"); // ""
        m.insert("rdoc", "\u{f48a}"); // ""
        m.insert("rds", "\u{f25d}"); // ""
        m.insert("readme", "\u{f48a}"); // ""
        m.insert("rlib", "\u{e7a8}"); // ""
        m.insert("rmd", "\u{f48a}"); // ""
        m.insert("rs", "\u{e7a8}"); // ""
        m.insert("rspec", "\u{e21e}"); // ""
        m.insert("rspec_parallel", "\u{e21e}"); // ""
        m.insert("rspec_status", "\u{e21e}"); // ""
        m.insert("rss", "\u{f09e}"); // ""
        m.insert("ru", "\u{e21e}"); // ""
        m.insert("rubydoc", "\u{e73b}"); // ""
        m.insert("sass", "\u{e603}"); // ""
        m.insert("scala", "\u{e737}"); // ""
        m.insert("scpt", "\u{f302}"); // ""
        m.insert("scss", "\u{e749}"); // ""
        m.insert("sh", "\u{f489}"); // ""
        m.insert("shell", "\u{f489}"); // ""
        m.insert("slim", "\u{e73b}"); // ""
        m.insert("sln", "\u{e70c}"); // ""
        m.insert("sql", "\u{f1c0}"); // ""
        m.insert("sqlite3", "\u{e7c4}"); // ""
        m.insert("styl", "\u{e600}"); // ""
        m.insert("stylus", "\u{e600}"); // ""
        m.insert("svg", "\u{f1c5}"); // ""
        m.insert("swift", "\u{e755}"); // ""
        m.insert("t", "\u{e769}"); // ""
        m.insert("tar", "\u{f410}"); // ""
        m.insert("tex", "\u{e600}"); // ""
        m.insert("tiff", "\u{f1c5}"); // ""
        m.insert("toml", "\u{e60b}"); // ""
        m.insert("torrent", "\u{f98c}"); // "歷"
        m.insert("ts", "\u{e628}"); // ""
        m.insert("tsx", "\u{e7ba}"); // ""
        m.insert("ttc", "\u{f031}"); // ""
        m.insert("ttf", "\u{f031}"); // ""
        m.insert("twig", "\u{e61c}"); // ""
        m.insert("txt", "\u{f15c}"); // ""
        m.insert("video", "\u{f03d}"); // ""
        m.insert("vim", "\u{e62b}"); // ""
        m.insert("vlc", "\u{f910}"); // "蘿"
        m.insert("vue", "\u{fd42}"); // "﵂"
        m.insert("wav", "\u{f001}"); // ""
        m.insert("webm", "\u{f03d}"); // ""
        m.insert("webp", "\u{f1c5}"); // ""
        m.insert("windows", "\u{f17a}"); // ""
        m.insert("wma", "\u{f001}"); // ""
        m.insert("wmv", "\u{f03d}"); // ""
        m.insert("wpl", "\u{f910}"); // "蘿"
        m.insert("woff", "\u{f031}"); // ""
        m.insert("woff2", "\u{f031}"); // ""
        m.insert("xls", "\u{f1c3}"); // ""
        m.insert("xlsx", "\u{f1c3}"); // ""
        m.insert("xml", "\u{e619}"); // ""
        m.insert("xul", "\u{e619}"); // ""
        m.insert("xz", "\u{f410}"); // ""
        m.insert("yaml", "\u{e60b}"); // ""
        m.insert("yml", "\u{e60b}"); // ""
        m.insert("zip", "\u{f410}"); // ""
        m.insert("zsh", "\u{f489}"); // ""
        m.insert("zsh-theme", "\u{f489}"); // ""
        m.insert("zshrc", "\u{f489}"); // ""

        m
    }
}

#[cfg(test)]
mod test {
    use super::{Icons, Theme};
    use crate::meta::Meta;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn get_no_icon() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path().join("file.txt");
        File::create(&file_path).expect("failed to create file");
        let meta = Meta::from_path(&file_path, false).unwrap();

        let icon = Icons::new(Theme::NoIcon, " ".to_string());
        let icon = icon.get(&meta.name);

        assert_eq!(icon, "");
    }

    #[test]
    fn get_default_file_icon() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path().join("file");
        File::create(&file_path).expect("failed to create file");
        let meta = Meta::from_path(&file_path, false).unwrap();

        let icon = Icons::new(Theme::Fancy, " ".to_string());
        let icon_str = icon.get(&meta.name);

        assert_eq!(icon_str, format!("{}{}", "\u{f016}", icon.icon_separator)); // 
    }

    #[test]
    fn get_default_file_icon_unicode() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path().join("file");
        File::create(&file_path).expect("failed to create file");
        let meta = Meta::from_path(&file_path, false).unwrap();

        let icon = Icons::new(Theme::Unicode, " ".to_string());
        let icon_str = icon.get(&meta.name);

        assert_eq!(icon_str, format!("{}{}", "\u{1f5cb}", icon.icon_separator));
    }

    #[test]
    fn get_directory_icon() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path();
        let meta = Meta::from_path(&file_path.to_path_buf(), false).unwrap();

        let icon = Icons::new(Theme::Fancy, " ".to_string());
        let icon_str = icon.get(&meta.name);

        assert_eq!(icon_str, format!("{}{}", "\u{f115}", icon.icon_separator)); // 
    }

    #[test]
    fn get_directory_icon_unicode() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path();
        let meta = Meta::from_path(&file_path.to_path_buf(), false).unwrap();

        let icon = Icons::new(Theme::Unicode, " ".to_string());
        let icon_str = icon.get(&meta.name);

        assert_eq!(icon_str, format!("{}{}", "\u{1f5c1}", icon.icon_separator));
    }

    #[test]
    fn get_directory_icon_with_ext() {
        let tmp_dir = tempdir().expect("failed to create temp dir");
        let file_path = tmp_dir.path();
        let meta = Meta::from_path(&file_path.to_path_buf(), false).unwrap();

        let icon = Icons::new(Theme::Fancy, " ".to_string());
        let icon_str = icon.get(&meta.name);

        assert_eq!(icon_str, format!("{}{}", "\u{f115}", icon.icon_separator)); // 
    }

    #[test]
    fn get_icon_by_name() {
        let tmp_dir = tempdir().expect("failed to create temp dir");

        for (file_name, file_icon) in &Icons::get_default_icons_by_name() {
            let file_path = tmp_dir.path().join(file_name);
            File::create(&file_path).expect("failed to create file");
            let meta = Meta::from_path(&file_path, false).unwrap();

            let icon = Icons::new(Theme::Fancy, " ".to_string());
            let icon_str = icon.get(&meta.name);

            assert_eq!(icon_str, format!("{}{}", file_icon, icon.icon_separator));
        }
    }

    #[test]
    fn get_icon_by_extension() {
        let tmp_dir = tempdir().expect("failed to create temp dir");

        for (ext, file_icon) in &Icons::get_default_icons_by_extension() {
            let file_path = tmp_dir.path().join(format!("file.{}", ext));
            File::create(&file_path).expect("failed to create file");
            let meta = Meta::from_path(&file_path, false).unwrap();

            let icon = Icons::new(Theme::Fancy, " ".to_string());
            let icon_str = icon.get(&meta.name);

            assert_eq!(icon_str, format!("{}{}", file_icon, icon.icon_separator));
        }
    }
}
#[cfg(test)]
mod tests_llm_16_225 {
    use std::collections::HashMap;
    use crate::icon::Icons;

    #[test]
    fn test_get_default_icons_by_extension() {
        let icons = Icons::get_default_icons_by_extension();
        assert_eq!(icons.get("7z"), Some(&"\u{f410}"));
        assert_eq!(icons.get("ai"), Some(&"\u{e7b4}"));
        assert_eq!(icons.get("apk"), Some(&"\u{e70e}"));
        assert_eq!(icons.get("avi"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("avro"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("awk"), Some(&"\u{f489}"));
        assert_eq!(icons.get("bash"), Some(&"\u{f489}"));
        assert_eq!(icons.get("bash_history"), Some(&"\u{f489}"));
        assert_eq!(icons.get("bash_profile"), Some(&"\u{f489}"));
        assert_eq!(icons.get("bashrc"), Some(&"\u{f489}"));
        assert_eq!(icons.get("bat"), Some(&"\u{f17a}"));
        assert_eq!(icons.get("bio"), Some(&"\u{f910}"));
        assert_eq!(icons.get("bmp"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("bz2"), Some(&"\u{f410}"));
        assert_eq!(icons.get("c"), Some(&"\u{e61e}"));
        assert_eq!(icons.get("c++"), Some(&"\u{e61d}"));
        assert_eq!(icons.get("cc"), Some(&"\u{e61d}"));
        assert_eq!(icons.get("cfg"), Some(&"\u{e615}"));
        assert_eq!(icons.get("clj"), Some(&"\u{e768}"));
        assert_eq!(icons.get("cljs"), Some(&"\u{e76a}"));
        assert_eq!(icons.get("cls"), Some(&"\u{e600}"));
        assert_eq!(icons.get("coffee"), Some(&"\u{f0f4}"));
        assert_eq!(icons.get("conf"), Some(&"\u{e615}"));
        assert_eq!(icons.get("cp"), Some(&"\u{e61d}"));
        assert_eq!(icons.get("cpp"), Some(&"\u{e61d}"));
        assert_eq!(icons.get("cs"), Some(&"\u{f81a}"));
        assert_eq!(icons.get("cshtml"), Some(&"\u{f1fa}"));
        assert_eq!(icons.get("csproj"), Some(&"\u{f81a}"));
        assert_eq!(icons.get("csx"), Some(&"\u{f81a}"));
        assert_eq!(icons.get("csh"), Some(&"\u{f489}"));
        assert_eq!(icons.get("css"), Some(&"\u{e749}"));
        assert_eq!(icons.get("csv"), Some(&"\u{f1c3}"));
        assert_eq!(icons.get("cxx"), Some(&"\u{e61d}"));
        assert_eq!(icons.get("d"), Some(&"\u{e7af}"));
        assert_eq!(icons.get("dart"), Some(&"\u{e798}"));
        assert_eq!(icons.get("db"), Some(&"\u{f1c0}"));
        assert_eq!(icons.get("diff"), Some(&"\u{f440}"));
        assert_eq!(icons.get("doc"), Some(&"\u{f1c2}"));
        assert_eq!(icons.get("dockerfile"), Some(&"\u{f308}"));
        assert_eq!(icons.get("docx"), Some(&"\u{f1c2}"));
        assert_eq!(icons.get("ds_store"), Some(&"\u{f179}"));
        assert_eq!(icons.get("dump"), Some(&"\u{f1c0}"));
        assert_eq!(icons.get("ebook"), Some(&"\u{e28b}"));
        assert_eq!(icons.get("editorconfig"), Some(&"\u{e615}"));
        assert_eq!(icons.get("ejs"), Some(&"\u{e618}"));
        assert_eq!(icons.get("elm"), Some(&"\u{e62c}"));
        assert_eq!(icons.get("env"), Some(&"\u{f462}"));
        assert_eq!(icons.get("eot"), Some(&"\u{f031}"));
        assert_eq!(icons.get("epub"), Some(&"\u{e28a}"));
        assert_eq!(icons.get("erb"), Some(&"\u{e73b}"));
        assert_eq!(icons.get("erl"), Some(&"\u{e7b1}"));
        assert_eq!(icons.get("exe"), Some(&"\u{f17a}"));
        assert_eq!(icons.get("ex"), Some(&"\u{e62d}"));
        assert_eq!(icons.get("exs"), Some(&"\u{e62d}"));
        assert_eq!(icons.get("fish"), Some(&"\u{f489}"));
        assert_eq!(icons.get("flac"), Some(&"\u{f001}"));
        assert_eq!(icons.get("flv"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("font"), Some(&"\u{f031}"));
        assert_eq!(icons.get("fpl"), Some(&"\u{f910}"));
        assert_eq!(icons.get("fs"), Some(&"\u{e7a7}"));
        assert_eq!(icons.get("fsx"), Some(&"\u{e7a7}"));
        assert_eq!(icons.get("fsi"), Some(&"\u{e7a7}"));
        assert_eq!(icons.get("gdoc"), Some(&"\u{f1c2}"));
        assert_eq!(icons.get("gemfile"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("gemspec"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("gform"), Some(&"\u{f298}"));
        assert_eq!(icons.get("gif"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("git"), Some(&"\u{f1d3}"));
        assert_eq!(icons.get("go"), Some(&"\u{e626}"));
        assert_eq!(icons.get("gradle"), Some(&"\u{e70e}"));
        assert_eq!(icons.get("gsheet"), Some(&"\u{f1c3}"));
        assert_eq!(icons.get("gslides"), Some(&"\u{f1c4}"));
        assert_eq!(icons.get("guardfile"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("gz"), Some(&"\u{f410}"));
        assert_eq!(icons.get("h"), Some(&"\u{f0fd}"));
        assert_eq!(icons.get("hbs"), Some(&"\u{e60f}"));
        assert_eq!(icons.get("heic"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("heif"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("heix"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("hpp"), Some(&"\u{f0fd}"));
        assert_eq!(icons.get("hs"), Some(&"\u{e777}"));
        assert_eq!(icons.get("htm"), Some(&"\u{f13b}"));
        assert_eq!(icons.get("html"), Some(&"\u{f13b}"));
        assert_eq!(icons.get("hxx"), Some(&"\u{f0fd}"));
        assert_eq!(icons.get("ico"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("image"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("iml"), Some(&"\u{e7b5}"));
        assert_eq!(icons.get("ini"), Some(&"\u{e615}"));
        assert_eq!(icons.get("ipynb"), Some(&"\u{e606}"));
        assert_eq!(icons.get("jar"), Some(&"\u{e204}"));
        assert_eq!(icons.get("java"), Some(&"\u{e204}"));
        assert_eq!(icons.get("jpeg"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("jpg"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("js"), Some(&"\u{e74e}"));
        assert_eq!(icons.get("json"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("jsx"), Some(&"\u{e7ba}"));
        assert_eq!(icons.get("jl"), Some(&"\u{e624}"));
        assert_eq!(icons.get("ksh"), Some(&"\u{f489}"));
        assert_eq!(icons.get("less"), Some(&"\u{e758}"));
        assert_eq!(icons.get("lhs"), Some(&"\u{e777}"));
        assert_eq!(icons.get("license"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("localized"), Some(&"\u{f179}"));
        assert_eq!(icons.get("lock"), Some(&"\u{f023}"));
        assert_eq!(icons.get("log"), Some(&"\u{f18d}"));
        assert_eq!(icons.get("lua"), Some(&"\u{e620}"));
        assert_eq!(icons.get("lz"), Some(&"\u{f410}"));
        assert_eq!(icons.get("m3u"), Some(&"\u{f910}"));
        assert_eq!(icons.get("m3u8"), Some(&"\u{f910}"));
        assert_eq!(icons.get("m4a"), Some(&"\u{f001}"));
        assert_eq!(icons.get("magnet"), Some(&"\u{f076}"));
        assert_eq!(icons.get("markdown"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("md"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("mjs"), Some(&"\u{e74e}"));
        assert_eq!(icons.get("mkd"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("mkv"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("mobi"), Some(&"\u{e28b}"));
        assert_eq!(icons.get("mov"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("mp3"), Some(&"\u{f001}"));
        assert_eq!(icons.get("mp4"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("mustache"), Some(&"\u{e60f}"));
        assert_eq!(icons.get("nix"), Some(&"\u{f313}"));
        assert_eq!(icons.get("npmignore"), Some(&"\u{e71e}"));
        assert_eq!(icons.get("opus"), Some(&"\u{f001}"));
        assert_eq!(icons.get("ogg"), Some(&"\u{f001}"));
        assert_eq!(icons.get("ogv"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("otf"), Some(&"\u{f031}"));
        assert_eq!(icons.get("pdf"), Some(&"\u{f1c1}"));
        assert_eq!(icons.get("pem"), Some(&"\u{f805}"));
        assert_eq!(icons.get("php"), Some(&"\u{e73d}"));
        assert_eq!(icons.get("pl"), Some(&"\u{e769}"));
        assert_eq!(icons.get("pls"), Some(&"\u{f910}"));
        assert_eq!(icons.get("pm"), Some(&"\u{e769}"));
        assert_eq!(icons.get("png"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("ppt"), Some(&"\u{f1c4}"));
        assert_eq!(icons.get("pptx"), Some(&"\u{f1c4}"));
        assert_eq!(icons.get("procfile"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("properties"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("ps1"), Some(&"\u{f489}"));
        assert_eq!(icons.get("psd"), Some(&"\u{e7b8}"));
        assert_eq!(icons.get("pxm"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("py"), Some(&"\u{e606}"));
        assert_eq!(icons.get("pyc"), Some(&"\u{e606}"));
        assert_eq!(icons.get("r"), Some(&"\u{f25d}"));
        assert_eq!(icons.get("rakefile"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rar"), Some(&"\u{f410}"));
        assert_eq!(icons.get("razor"), Some(&"\u{f1fa}"));
        assert_eq!(icons.get("rb"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rdata"), Some(&"\u{f25d}"));
        assert_eq!(icons.get("rdb"), Some(&"\u{e76d}"));
        assert_eq!(icons.get("rdoc"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("rds"), Some(&"\u{f25d}"));
        assert_eq!(icons.get("readme"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("rlib"), Some(&"\u{e7a8}"));
        assert_eq!(icons.get("rmd"), Some(&"\u{f48a}"));
        assert_eq!(icons.get("rs"), Some(&"\u{e7a8}"));
        assert_eq!(icons.get("rspec"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rspec_parallel"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rspec_status"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rss"), Some(&"\u{f09e}"));
        assert_eq!(icons.get("ru"), Some(&"\u{e21e}"));
        assert_eq!(icons.get("rubydoc"), Some(&"\u{e73b}"));
        assert_eq!(icons.get("sass"), Some(&"\u{e603}"));
        assert_eq!(icons.get("scala"), Some(&"\u{e737}"));
        assert_eq!(icons.get("scpt"), Some(&"\u{f302}"));
        assert_eq!(icons.get("scss"), Some(&"\u{e749}"));
        assert_eq!(icons.get("sh"), Some(&"\u{f489}"));
        assert_eq!(icons.get("shell"), Some(&"\u{f489}"));
        assert_eq!(icons.get("slim"), Some(&"\u{e73b}"));
        assert_eq!(icons.get("sln"), Some(&"\u{e70c}"));
        assert_eq!(icons.get("sql"), Some(&"\u{f1c0}"));
        assert_eq!(icons.get("sqlite3"), Some(&"\u{e7c4}"));
        assert_eq!(icons.get("styl"), Some(&"\u{e600}"));
        assert_eq!(icons.get("stylus"), Some(&"\u{e600}"));
        assert_eq!(icons.get("svg"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("swift"), Some(&"\u{e755}"));
        assert_eq!(icons.get("t"), Some(&"\u{e769}"));
        assert_eq!(icons.get("tar"), Some(&"\u{f410}"));
        assert_eq!(icons.get("tex"), Some(&"\u{e600}"));
        assert_eq!(icons.get("tiff"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("toml"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("torrent"), Some(&"\u{f98c}"));
        assert_eq!(icons.get("ts"), Some(&"\u{e628}"));
        assert_eq!(icons.get("tsx"), Some(&"\u{e7ba}"));
        assert_eq!(icons.get("ttc"), Some(&"\u{f031}"));
        assert_eq!(icons.get("ttf"), Some(&"\u{f031}"));
        assert_eq!(icons.get("twig"), Some(&"\u{e61c}"));
        assert_eq!(icons.get("txt"), Some(&"\u{f15c}"));
        assert_eq!(icons.get("video"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("vim"), Some(&"\u{e62b}"));
        assert_eq!(icons.get("vlc"), Some(&"\u{f910}"));
        assert_eq!(icons.get("vue"), Some(&"\u{fd42}"));
        assert_eq!(icons.get("wav"), Some(&"\u{f001}"));
        assert_eq!(icons.get("webm"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("webp"), Some(&"\u{f1c5}"));
        assert_eq!(icons.get("windows"), Some(&"\u{f17a}"));
        assert_eq!(icons.get("wma"), Some(&"\u{f001}"));
        assert_eq!(icons.get("wmv"), Some(&"\u{f03d}"));
        assert_eq!(icons.get("wpl"), Some(&"\u{f910}"));
        assert_eq!(icons.get("woff"), Some(&"\u{f031}"));
        assert_eq!(icons.get("woff2"), Some(&"\u{f031}"));
        assert_eq!(icons.get("xls"), Some(&"\u{f1c3}"));
        assert_eq!(icons.get("xlsx"), Some(&"\u{f1c3}"));
        assert_eq!(icons.get("xml"), Some(&"\u{e619}"));
        assert_eq!(icons.get("xul"), Some(&"\u{e619}"));
        assert_eq!(icons.get("xz"), Some(&"\u{f410}"));
        assert_eq!(icons.get("yaml"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("yml"), Some(&"\u{e60b}"));
        assert_eq!(icons.get("zip"), Some(&"\u{f410}"));
        assert_eq!(icons.get("zsh"), Some(&"\u{f489}"));
        assert_eq!(icons.get("zsh-theme"), Some(&"\u{f489}"));
        assert_eq!(icons.get("zshrc"), Some(&"\u{f489}"));
    }
}#[cfg(test)]
mod tests_llm_16_228 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let theme = Theme::Fancy;
        let icon_separator = String::from("_");
        let icons = Icons::new(theme, icon_separator);
        assert_eq!(icons.display_icons, true);
        assert_eq!(icons.icon_separator, "_");
        // assert other values
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_rug() {
        let expected: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();

            // Note: filenames must be lower-case

            m.insert(".trash", "\u{f1f8}"); // ""
            m.insert(".atom", "\u{e764}"); // ""
            m.insert(".bashprofile", "\u{e615}"); // ""
            m.insert(".bashrc", "\u{f489}"); // ""
            m.insert(".clang-format", "\u{e615}"); // ""
            m.insert(".git", "\u{f1d3}"); // ""
            m.insert(".gitattributes", "\u{f1d3}"); // ""
            m.insert(".gitconfig", "\u{f1d3}"); // ""
            m.insert(".github", "\u{f408}"); // ""
            m.insert(".gitignore", "\u{f1d3}"); // ""
            m.insert(".gitmodules", "\u{f1d3}"); // ""
            m.insert(".rvm", "\u{e21e}"); // ""
            m.insert(".vimrc", "\u{e62b}"); // ""
            m.insert(".vscode", "\u{e70c}"); // ""
            m.insert(".zshrc", "\u{f489}"); // ""
            m.insert("bin", "\u{e5fc}"); // ""
            m.insert("config", "\u{e5fc}"); // ""
            m.insert("docker-compose.yml", "\u{f308}"); // ""
            m.insert("dockerfile", "\u{f308}"); // ""
            m.insert("ds_store", "\u{f179}"); // ""
            m.insert("gitignore_global", "\u{f1d3}"); // ""
            m.insert("gradle", "\u{e70e}"); // ""
            m.insert("gruntfile.coffee", "\u{e611}"); // ""
            m.insert("gruntfile.js", "\u{e611}"); // ""
            m.insert("gruntfile.ls", "\u{e611}"); // ""
            m.insert("gulpfile.coffee", "\u{e610}"); // ""
            m.insert("gulpfile.js", "\u{e610}"); // ""
            m.insert("gulpfile.ls", "\u{e610}"); // ""
            m.insert("hidden", "\u{f023}"); // ""
            m.insert("include", "\u{e5fc}"); // ""
            m.insert("lib", "\u{f121}"); // ""
            m.insert("localized", "\u{f179}"); // ""
            m.insert("node_modules", "\u{e718}"); // ""
            m.insert("npmignore", "\u{e71e}"); // ""
            m.insert("rubydoc", "\u{e73b}"); // ""

            m
        };

        let result = Icons::get_default_icons_by_name();
        
        assert_eq!(result, expected);
    }
}
                                  