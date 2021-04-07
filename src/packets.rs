use serde::Deserialize;
use serde::Serialize;

/// Send a message informing the cloud the machine is active.
#[derive(Serialize, Deserialize)]
pub struct BootRequest {
    pub student_number: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CommandResponse {
    pub ok: bool,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ShutdownRequest {
    pub name: String,
    pub student_number: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JobRequestPacket {
    pub student_number: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Job {
    pub number: u64,
    pub size: u64,
    pub nounce_start: u64,
    pub nounce_end: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JobResponsePacket {
    Success(Job),
    Error(String),
}

/// Solution info 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Solution {
    pub sha256: String,
    pub nounce: String,
    pub time: f64,
}

/// When the job is complete, this packet is sent to the pool.
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmittionPacket {
    pub job_n: u64,
    pub name: String,
    pub student_number: String,
    pub thread_hashes_per_second: f64,
    pub total_hashes_per_second: f64,
    pub nounce_start: u64,
    pub nounce_end: u64,
    pub solutions: Vec<Solution>,
}

/// Received from the server on job submission.
#[derive(Serialize, Deserialize, Debug)]
pub enum SubmittionResponsePacket {
    Accepted,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolStatusRequestPacket {
    pub student_number: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PoolStatusResponsePacket {
    pub user_total_hash_rate: f64,
    pub user_total_shares: usize,
    pub pool_total_shares: usize,
    pub pool_best_zero_length: u8,
    pub completed_jobs: u64,
}