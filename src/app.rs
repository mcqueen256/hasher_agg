use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use crate::submitter::Submitter;
use crate::file_operations::{
    get_cwd,
    open_append_file,
    open_overwrite_file,
    open_read_file,
};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BestSolution {
    pub student_number: String,
    pub job_number: u64,
    pub leading_zero_bit_length: u8,
    pub nounce: String,
    pub hash: String,
}

#[derive(Debug)]
pub struct ApplicationData {
    pub submitters: HashMap<String, Submitter>,
    pub best: Option<BestSolution>,
}

#[derive(PartialEq, Eq)]
pub enum HashSubmittion {
    Accepted,
    AlreadyExists,
}

impl ApplicationData {
    pub fn begin() -> Self {
        let mut best = None;
        let mut submitters = HashMap::new();
        if let Ok(file) = open_read_file("best", "best.json") {
            if let Ok(best_from_file) = serde_json::from_reader(&file) {
                best = Some(best_from_file);
            }
        }
        let str_path = format!("{}/data/submitters", get_cwd());
        if let Ok(paths) = std::fs::read_dir(&str_path) {
            for path in  paths {
                let path = path.unwrap().path();
                let str_path = format!("{}", path.display());
                let student_number = str_path.split("/").last().unwrap();
                if let Ok(file) = open_read_file(
                    &format!("submitters/{}", student_number),
                    "info.json"
                ) {
                    if let Ok(submitter) = serde_json::from_reader(file) {
                        submitters.insert(String::from(student_number), submitter);
                    }
                }

            }
        }
        ApplicationData {
            submitters,
            best,
        }
    }

    pub fn submitter_from<'a>(&'a mut self, student_number: &str) -> &'a mut Submitter {
        if let None = self.submitters.get(student_number) {
            let submitter = Submitter::new(student_number);
            self.submitters.insert(String::from(student_number), submitter);
        }
        self.submitters.get_mut(student_number).unwrap()
    }

    pub fn submit_hash(&mut self, hash: &String) -> HashSubmittion {
        use std::io::BufRead;
        use std::io::prelude::*;
        // below line may fail if first time.
        if let Ok(file) = open_read_file("hashes", "hashes.txt") {
            for line in std::io::BufReader::new(file).lines() {
                if let Ok(line) = line {
                    if line.eq(hash) {
                        return HashSubmittion::AlreadyExists 
                    }
                }
            }
        }
        let mut file = open_append_file("hashes", "hashes.txt")
            .expect("Could not open data/hashes.txt for writing");
        let _ = writeln!(file, "{}", hash);
        HashSubmittion::Accepted
    }


    pub fn save_best(&self, best: BestSolution) {
        let file  = open_overwrite_file("best","best.json",
            ).expect("Could not open/overwrite best solution file.");
        serde_json::to_writer(&file, &best)
            .expect("Counld not write JSON to best solution file");
    }
}

