use args::Args;
use cargo_edit::Manifest;

static DEFAULT_CARGO_TOML: &'static str = r#"[package]
authors = ["Some Guy"]
name = "lorem-ipsum"
version = "0.1.0"

[dependencies]
foo-bar = "0.1""#;

macro_rules! add_deps_test {
    ($name:ident: add $crate_name:expr => $section:expr) => {
        #[test]
        fn $name() {
            let opts = Args {
                arg_crate: String::from($crate_name),
                flag_dev: $section.contains("dev-"),
                flag_build: $section.contains("build-"),
                ..Default::default()
            };

            let mut manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

            manifile.insert_into_table(
                &opts.get_section(),
                &opts.parse_dependency().expect("Error parsing dependency")
            ).unwrap();

            let entry = manifile.data.get(opts.get_section()).expect("section not found")
                                     .lookup($crate_name).expect("entry not found")
                                     .as_str().expect("entry not a str");

            assert_eq!(entry, String::from("*"));
        }
    };
    ($name:ident: add $crate_name:expr, version $version:expr => $section:expr) => {
        #[test]
        fn $name() {
            let opts = Args {
                arg_crate: String::from($crate_name),
                flag_dev: $section.contains("dev-"),
                flag_build: $section.contains("build-"),
                flag_ver: Some(String::from($version)),
                ..Default::default()
            };

            let mut manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

            manifile.insert_into_table(
                &opts.get_section(),
                &opts.parse_dependency().expect("Error parsing dependency")
            ).unwrap();

            let entry = manifile.data.get(opts.get_section()).expect("section not found")
                                     .lookup($crate_name).expect("entry not found")
                                     .as_str().expect("entry not a str");

            assert_eq!(entry, $version);
        }
    };
}

add_deps_test!(add_dependency:         add "lorem-ipsum" => "dependencies");
add_deps_test!(add_dependency_version: add "lorem-ipsum", version "0.4.2" => "dependencies");
add_deps_test!(add_dev_dependency:     add "lorem-ipsum", version "0.4.2" => "dev-dependencies");
add_deps_test!(add_build_dependency:   add "lorem-ipsum", version "0.4.2" => "build-dependencies");

#[test]
fn add_dependency_from_git() {
    let opts = Args {
        arg_crate: String::from("amet"),
        flag_dev: true,
        flag_git: Some(String::from("https://localhost/amet.git")),
        ..Default::default()
    };

    let mut manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

    manifile.insert_into_table(opts.get_section(),
                               &opts.parse_dependency().expect("Error parsing dependency"))
            .unwrap();

    let entry = manifile.data
                        .get(opts.get_section()).expect("section not found")
                        .lookup("amet").expect("entry not found")
                        .lookup("git").expect("git not found")
                        .as_str().expect("entry not a str");

    assert_eq!(entry, "https://localhost/amet.git");
}

#[test]
fn add_dependency_from_path() {
    let opts = Args {
        arg_crate: String::from("amet"),
        flag_dev: true,
        flag_path: Some(String::from("../amet")),
        ..Default::default()
    };

    let mut manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

    manifile.insert_into_table(&opts.get_section(),
                               &opts.parse_dependency().expect("Error parsing dependency"))
            .unwrap();

    let entry = manifile.data
                        .get(opts.get_section())
                        .expect("section not found")
                        .lookup("amet")
                        .expect("entry not found")
                        .lookup("path")
                        .expect("path not found")
                        .as_str()
                        .expect("entry not a str");

    assert_eq!(entry, "../amet");
}
