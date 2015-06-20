use args::Args;
use manifest::Manifest;

static DEFAULT_CARGO_TOML: &'static str = r#"[package]
authors = ["Some Guy"]
name = "lorem-ipsum"
version = "0.1.0"

[dependencies]
foo-bar = "0.1""#;

macro_rules! add_deps_test {
    ($name:ident: add $entry:expr => $section:expr) => {
        #[test]
        fn $name() {
            let opts = Args {
                arg_section: String::from($section),
                arg_dep: vec![String::from($entry)],
                ..Default::default()
            };

            let mut manifile = Manifest::from_str(DEFAULT_CARGO_TOML).unwrap();

            manifile.add_deps(
                &opts.get_section(),
                &opts.get_dependencies()
            ).unwrap();

            let entry = manifile.data.get(&opts.get_section()).expect("section not found")
                .lookup($entry).expect("entry not found")
                .as_str().expect("entry not a str");

            assert_eq!(
                entry,
                "*"
            )
        }
    };

    ($name:ident: add $entry:expr, version $version:expr => $section:expr) => {
        #[test]
        fn $name() {
            let opts = Args {
                arg_section: String::from($section),
                arg_dep: vec![String::from($entry)],
                arg_source: String::from($version),
                flag_version: true,
                ..Default::default()
            };

            let mut manifile = Manifest::from_str(DEFAULT_CARGO_TOML).unwrap();

            manifile.add_deps(
                &opts.get_section(),
                &opts.get_dependencies()
            ).unwrap();

            let entry = manifile.data.get(&opts.get_section()).expect("section not found")
                .lookup($entry).expect("entry not found")
                .as_str().expect("entry not a str");

            assert_eq!(
                entry,
                $version
            )
        }
    }
}

add_deps_test!(add_dependency: add "lorem-ipsum" => "dependencies");
add_deps_test!(add_dep: add "lorem-ipsum" => "deps");
add_deps_test!(add_dev_dependency: add "lorem-ipsum" => "dev-dependencies");
add_deps_test!(add_dev_dep: add "lorem-ipsum" => "dev-deps");
add_deps_test!(add_build_dependency: add "lorem-ipsum" => "build-dependencies");
add_deps_test!(add_build_dep: add "lorem-ipsum" => "build-deps");

add_deps_test!(add_dependency_version: add "lorem-ipsum", version "0.4.2" => "dependencies");
