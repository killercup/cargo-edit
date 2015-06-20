macro_rules! toml_table {
    ($key:expr => $value:expr) => {
        {
            let mut dep = BTreeMap::new();
            dep.insert(String::from($key), toml::Value::String($value.clone()));
            toml::Value::Table(dep)
        }
    }
}
