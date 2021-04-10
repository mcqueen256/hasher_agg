use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

use crate::file_operations::{
    get_cwd,
    open_read_file,
    open_overwrite_file,
    open_append_file,
};
use crate::packets::{Job, Solution};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Machine {
    pub name: String,
    pub reported_thread_hashrate: f64,
    pub reported_thread_hashrate_history: Vec<f64>,
    pub reported_total_hashrate: f64,
    pub reported_total_hashrate_history: Vec<f64>,
    pub calculated_job_size: u64,
    pub online: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Submitter {
    pub student_number: String,
    pub next_job_number: u64,
    pub next_nounce: u64,
    pub pending_jobs: Vec<StoredJob>,
    pub unfinished_jobs: Vec<StoredJob>,
    pub accepted_shares_count: u64,
    pub machines: Vec<Machine>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct StoredJob {
    pub number: u64,
    pub size: u64,
    pub nounce_start: u64,
    pub nounce_end: u64,
    pub quote_time: f64,
}



impl Submitter {
    pub fn new(student_number: &str) -> Self {
        let path = &format!("{}/data/submitters/{}/info.json", get_cwd(), student_number);
        let open_path = &format!("{}/data/submitters/{}", get_cwd(), student_number);
        if Path::new(path).exists() {
            if let Ok(file) = open_read_file(open_path, "info.json") {
                return serde_json::from_reader(&file).expect("Could not interpret json");
            }
        }
        
        let submitter = Self {
            machines: vec![],
            next_job_number: 0,
            pending_jobs: vec![],
            unfinished_jobs: vec![],
            accepted_shares_count: 0,
            next_nounce: 0,
            student_number: String::from(student_number),
        };
        submitter.save();
        submitter
    }

    /// Returns the machine with the given name. If no machine exists, a new one is made.
    pub fn get_machine<'a>(&'a mut self, name: &str) -> &'a mut Machine {
        let found = self.machines.iter()
            .enumerate()
            .find_map(|(i, m)| {
                if m.name.eq(name) {
                    Some(i)
                } else {
                    None
                }
            });
        if let Some(index) = found {
            self.machines.get_mut(index).unwrap()
        } else {
            // No machine found at this point. Make a new one.
            let machines = &mut self.machines;
            let machine = Machine {
                name: String::from(name),
                reported_thread_hashrate: 0.0,
                reported_thread_hashrate_history: vec![],
                reported_total_hashrate: 0.0,
                reported_total_hashrate_history: vec![],
                calculated_job_size: 1_000_000,
                online: true,
            };
            machines.push(machine);
            machines.last_mut().unwrap()
        }

    }

    pub fn next_job(&mut self, name: &str) -> Job {
        // Check if we need to add old jobs to the unfinished list.
        let mut old_job_indexes = vec![];
        for (i, pending) in self.pending_jobs.iter().enumerate() {
            let age = crate::util::get_time() - pending.quote_time;
            if age > 10.0*60.0 {
                old_job_indexes.push(i);
            }
        }
        // Move old jobs to unfinished.
        for &index in old_job_indexes.iter().rev() {
            self.unfinished_jobs.push(self.pending_jobs.remove(index));
        }
        // If there are jobs that have not been processed, then process them.
        if let Some(job) = self.unfinished_jobs.pop() {
            self.pending_jobs.push(job.clone());
            let job = Job {
                number: job.number,
                nounce_start: job.nounce_start,
                nounce_end: job.nounce_end,
                size: job.size,
            };
            return job;
        }
        // Make new job.
        let number = self.next_job_number;
        self.next_job_number += 1;
        let machine = self.get_machine(name);
        let size = machine.calculated_job_size;
        let nounce_start = self.next_nounce;
        let nounce_end = nounce_start + size;
        self.next_nounce = nounce_end;
        let job = StoredJob {
            number,
            size,
            nounce_start,
            nounce_end,
            quote_time: crate::util::get_time(),
        };
        self.pending_jobs.push(job.clone());

        let job = Job {
            number: job.number,
            nounce_start: job.nounce_start,
            nounce_end: job.nounce_end,
            size: job.size,
        };
        return job;
    }

    pub fn pop_pending_job(&mut self, number: u64) -> Result<Job, ()> {
        let mut some_index = None;
        for (i, job) in self.pending_jobs.iter().enumerate() {
            if job.number == number {
                some_index = Some(i);
                break;
            }
        }
        if let Some(index) = some_index {
            let job = self.pending_jobs.remove(index);
            let job = Job {
                number: job.number,
                nounce_start: job.nounce_start,
                nounce_end: job.nounce_end,
                size: job.size,
            };
            Ok(job)
        } else {
            Err(())
        }
    }

    pub fn user_hash_rate(&self) -> f64 {
        let mut sum = 0.0;
        for machine in self.machines.iter() {
            sum += machine.reported_total_hashrate;
        }
        sum / self.machines.len() as f64
    }

    pub fn save(&self) {
        let file  = open_overwrite_file(
        &format!("submitters/{}", self.student_number),
        "info.json"
        ).expect("Could not open/overwrite submitters, info JSON file.");
        serde_json::to_writer(&file, &self)
        .expect(&format!("Counld not write JSON to submitters/{}/info.json", self.student_number));
    }

    pub fn save_solution(&self, solution: Solution, leading_zero_bits_length: u8) {
        let file  = open_append_file(
            &format!("submitters/{}", self.student_number),
            &format!("sol_{:02}", leading_zero_bits_length),
            ).expect("Could not open/append submitters solution file.");
        serde_json::to_writer(&file, &solution)
            .expect("Counld not write JSON to submitters solution file");
        use std::io::Write;
        writeln!(&file, "").expect("Counld not write line JSON to submitters solution file");
    }
}
