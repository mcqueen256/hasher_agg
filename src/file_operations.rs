use std::fs::OpenOptions;

fn make_path(path: &str) {
    let dir_path = format!("{}/{}/{}", get_cwd(), "data", path);
    let err_msg = format!("could not create directory: {}", dir_path);
    std::fs::create_dir_all(&dir_path).expect(&err_msg);
}

pub fn get_cwd() -> String {
    let cwd = std::env::current_dir().expect("could not get cwd.");
    let cwd = cwd.display();
    format!("{}", cwd)
}

pub fn open_append_file(path: &str, filename: &str) -> Result<std::fs::File, std::io::Error> {
    make_path(path);
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&format!("{}/data/{}/{}", get_cwd(), path, filename))
}

pub fn open_overwrite_file(path: &str, filename: &str) -> Result<std::fs::File, std::io::Error> {
    make_path(path);
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&format!("{}/data/{}/{}", get_cwd(), path, filename))
}

pub fn open_read_file(path: &str, filename: &str) -> Result<std::fs::File, std::io::Error> {
    make_path(path);
    OpenOptions::new()
        .read(true)
        .open(&format!("{}/data/{}/{}", get_cwd(), path, filename))
}