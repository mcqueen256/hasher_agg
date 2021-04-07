use std::collections::HashMap;
use crate::packets::{Job, Solution};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
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
    pub pending_jobs: Vec<Job>,
    pub unfinished_jobs: Vec<Job>,
    pub accepted_shares: Vec<Solution>,
    pub rejected_shares: Vec<Solution>,
    pub machines: Vec<Machine>,
}

pub struct StoredJob {
    pub number: u64,
    pub size: u64,
    pub nounce_start: u64,
    pub nounce_end: u64,
    pub quote_time: f64,
}

impl Submitter {

    pub fn new(student_number: &str) -> Self {
        Self {
            machines: vec![],
            next_job_number: 0,
            pending_jobs: vec![],
            unfinished_jobs: vec![],
            accepted_shares: vec![],
            rejected_shares: vec![],
            next_nounce: 0,
            student_number: String::from(student_number),
        }
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
        // If there are jobs that have not been processed, then process them.
        if let Some(job) = self.unfinished_jobs.pop() {
            self.pending_jobs.push(job.clone());
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
        let job = Job {
            number,
            size,
            nounce_start,
            nounce_end,
        };
        self.pending_jobs.push(job.clone());
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
            Ok(self.pending_jobs.remove(index))
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BestSolution {
    pub student_number: String,
    pub job_number: u64,
    pub leading_zero_bit_length: u8,
    pub nounce: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationData {
    pub submitters: HashMap<String, Submitter>,
    pub hashes: Vec<String>,
    pub best: Option<BestSolution>,
}

#[derive(PartialEq, Eq)]
pub enum HashSubmittion {
    Accepted,
    AlreadyExists,
}

impl ApplicationData {
    pub fn begin() -> Self {
        ApplicationData {
            submitters: HashMap::new(),
            hashes: vec![],
            best: None,
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
        if self.hashes.contains(&hash) {
            HashSubmittion::AlreadyExists
        } else {
            self.hashes.push(hash.clone());
            HashSubmittion::Accepted
        }
    }
}