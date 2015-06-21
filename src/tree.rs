use std::error::Error;
use std::collections::BTreeMap;
use std::iter::repeat;

use toml;

use manifest::Manifest;
use list_error::ListError;

type PkgName = String;
type PkgVersion = String;
type Package = (PkgName, PkgVersion);
type Dependency = Package;
type Dependencies = Vec<Dependency>;

pub type Packages = BTreeMap<Package, Dependencies>;

/// Parse stuff like `"docopt 0.6.67 (registry+https://github.com/rust-lang/crates.io-index)"`
/// by splitting at whitespace and taking the first two things.
fn parse_dep_from_str(input: &str) -> Option<Dependency> {
    let pkg = input.split(' ').collect::<Vec<&str>>();
    if pkg.len() != 3 { return None; }
    Some((
        String::from(pkg[0]), // name
        String::from(pkg[1]), // version
    ))
}

fn get_root_deps(lock_file: &toml::Table) -> Result<Vec<Dependency>, Box<Error>> {
    let root_deps = try!(
        lock_file
        .get("root")
        .and_then(|field| field.lookup("dependencies"))
        .ok_or(ListError::SectionMissing("root.dependencies".to_owned()))
    );

    let output = root_deps.as_slice()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|dep| {
            let dep = dep.clone();
            if let toml::Value::String(pkg_desc) = dep {
                parse_dep_from_str(&pkg_desc)
            } else {
                None
            }
        })
        .collect::<Vec<Dependency>>();

    Ok(output)
}

fn get_packages(lock_file: &toml::Table) -> Result<Packages, Box<Error>> {
    let packages: &toml::Value = try!(
        lock_file
        .get("package")
        .ok_or(ListError::SectionMissing("package".to_owned()))
    );

    let mut output = BTreeMap::new();

    for pkg in packages.as_slice().unwrap_or(&vec![]) {
        let package = try!(pkg.as_table()
            .ok_or(ListError::SectionMissing("package".to_owned())));

        let name = try!(package.get("name")
            .and_then(|item| item.as_str())
            .ok_or(ListError::SectionMissing("name".to_owned())));

        let version = try!(package.get("version")
            .and_then(|item| item.as_str())
            .ok_or(ListError::SectionMissing("version".to_owned())));

        let deps: Dependencies = package.get("dependencies")
            .and_then(|item| {
                let item = item.clone();
                if let toml::Value::Array(d) = item {
                    Some(d)
                } else { None }
            })
            .and_then(|items| Some(items.iter()
                .filter_map(|i| i.as_str())
                .filter_map(parse_dep_from_str)
                .collect::<Dependencies>()))
            .unwrap_or(vec![]);

        output.insert((name.to_owned(), version.to_owned()), deps);
    }

    Ok(output)
}

const INDENT: u32 = 4;

fn list_deps(pkgs: &Packages, deps: &Dependencies, level: u32) -> Result<String, Box<Error>> {
    let mut output = String::new();
    for dep in deps {
        output.push_str(&repeat(" ").take((level * INDENT) as usize).collect::<String>());
        output.push_str(&format!("‣ {} ({})\n", dep.0, dep.1));

        if let Some(subdeps) = pkgs.get(dep) {
            let sublist = try!(list_deps(pkgs, subdeps, level + 1));
            output.push_str(&sublist);
        }
    }
    Ok(output)
}

pub fn parse_lock_file(manifest: &Manifest) -> Result<String, Box<Error>> {
    let lock_file = &manifest.data;

    let root_deps = try!(get_root_deps(lock_file));
    let pkgs = try!(get_packages(lock_file));

    list_deps(&pkgs, &root_deps, 0)
}

#[cfg(test)]
mod test {
    use manifest::Manifest;
    use super::parse_lock_file;

    #[test]
    fn basic_tree() {
        let manifile = Manifest::open_lock_file(
            &Some(&"tests/fixtures/tree/Cargo.lock".to_owned())
        ).unwrap();

        assert_eq!(
            parse_lock_file(&manifile).unwrap(),
            "\
‣ clippy (0.0.5)
‣ docopt (0.6.67)
    ‣ regex (0.1.38)
        ‣ aho-corasick (0.2.1)
            ‣ memchr (0.1.3)
                ‣ libc (0.1.8)
        ‣ memchr (0.1.3)
            ‣ libc (0.1.8)
        ‣ regex-syntax (0.1.2)
    ‣ rustc-serialize (0.3.15)
    ‣ strsim (0.3.0)
‣ pad (0.1.4)
    ‣ unicode-width (0.1.1)
‣ rustc-serialize (0.3.15)
‣ semver (0.1.19)
‣ toml (0.1.20)
    ‣ rustc-serialize (0.3.15)
"
        );
    }
}
