use std::sync::{Arc, Mutex};
use sha2::{Digest, Sha256};
use actix_web::{web, get, post, HttpResponse, Responder, web::Json};
use crate::{app::{ApplicationData, HashSubmittion, BestSolution}, packets};
use crate::submitter::StoredJob;

type AppData = web::Data<Arc<Mutex<ApplicationData>>>;

#[get("/")]
pub async fn index(data: AppData) -> impl Responder {
    let app = data.lock().unwrap();

    let mut body = String::new();
    body += "<!DOCTYPE html><html><head></head><body>";
    if let Some(best) = &app.best {
        body += &format!("
            <h1>Best Solution</h1>
            <p>Leading zero bits length: <b>{}</b></p>
            <p>Found by: <b>{}</b></p>
            <p>nounce: <b>{}</b></p>
            <p>hash: <b>{}</b></p>
            ",
            best.leading_zero_bit_length,
            best.student_number,
            best.nounce,
            best.hash,
        );
    } else {
        body += &format!("<h1>No Best Solution Yet<h1>");
    }

    body += "<h2>Submitters</h2>";
    let pool_total_shares: usize = app.submitters.iter()
        .map(|(_student_number, submitter)| submitter.accepted_shares_count as usize)
        .sum();

    for (student_number, submitter) in app.submitters.iter() {
        let user_total_shares = submitter.accepted_shares_count as usize;
        body += &format!("
            <p>SN: <b>{}</b></p>
            <p>MH/s: <b>{}</b></p>
            <p>shares: <b>{}/{}</b></p>
            <hr />
            ",
            student_number,
            submitter.user_hash_rate() / 1_000_000.0,
            user_total_shares,
            pool_total_shares,
        );
    }
    body += "</body>";
    
    HttpResponse::Ok()
    .content_type("text/html")
    .body(&body)
}

#[post("/boot")]
pub async fn boot(data: AppData, boot_request: Json<packets::BootRequest>) -> impl Responder {
    let mut app = data.lock().unwrap();
    let submitter = (*app).submitter_from(&boot_request.student_number);
    let machine = submitter.get_machine(&boot_request.name);
    machine.online = true;
    submitter.save();
    HttpResponse::Ok().json(packets::CommandResponse { ok: true, msg: None })
}


#[post("/shutdown")]
pub async fn showdown(data: AppData, shutdown_request: Json<packets::ShutdownRequest>) -> impl Responder {
    let mut app = data.lock().unwrap();
    let submitter = (*app).submitter_from(&shutdown_request.student_number);
    let machine = submitter.get_machine(&shutdown_request.name);
    machine.online = false;
    submitter.save();
    HttpResponse::Ok().json(packets::CommandResponse { ok: true, msg: None })
}

#[post("/job/request")]
pub async fn job_request(data: AppData, job_request: Json<packets::JobRequestPacket>) -> impl Responder {
    let mut app = data.lock().unwrap();
    let submitter = (*app).submitter_from(&job_request.student_number);
    let job = submitter.next_job(&job_request.name);
    submitter.save();
    HttpResponse::Ok().json(packets::JobResponsePacket::Success(job))
}

#[post("/job/submit")]
pub async fn job_submit(data: AppData, submit_request: Json<packets::SubmittionPacket>) -> impl Responder {

    let mut app = data.lock().unwrap();

    let pending_job = {
        let submitter = app.submitter_from(&submit_request.student_number);
        if let Ok(job) = submitter.pop_pending_job(submit_request.job_n) {
            job 
        } else {
            eprintln!("!/job/submit: no pending job. {}", submit_request.job_n);
            return HttpResponse::Ok().json(packets::SubmittionResponsePacket::Rejected); // no pending job.
        }
        // Job was pending!
    };

    // Check if the start range is the same as the pending job.
    if pending_job.nounce_start != submit_request.nounce_start {
        eprintln!("invalid nounce_start.");
        return HttpResponse::Ok().json(packets::SubmittionResponsePacket::Rejected); // invalid nounce_start.
    }

    // Collect valid & in-valid hashes
    let mut valid_solutions = Vec::new();
    let mut sh = Sha256::default();
    for sol in submit_request.solutions.iter() {
        let buffer = if let Ok(buffer) = hash_to_sha256_buffer(&sol.sha256) { buffer } else {
            eprintln!("Something is wrong with the hash");
            continue; // Something is wrong with the hash.
        };
        let leading_zero_bits = count_leading_zero_bits(&buffer);
        // Check is hash length requirement passes
        if leading_zero_bits < crate::constants::MINIMUN_ZERO_BIT_LENGTH {
            println!("buffer: {:?}", buffer);
            eprintln!("hash length requirement failed: {}", leading_zero_bits);
            continue; // hash length requirement failed.
        }

        // Check the hash is true.
        let mut buffer: Vec<u8> = vec![]; // To hash.
        // Add student number to buffer.
        let student_number = submit_request.student_number.clone();
        student_number.chars().for_each(|c| buffer.push(c as u8));
        // Add Initial nounce to buffer.
        for c in sol.nounce.chars() {
            buffer.push(c as u8);
        }
        //calc hash
        sh.update(&buffer);
        let sha256_buffer = sh.finalize_reset();
        let hash = sha245_to_string(&sha256_buffer);
        if !hash.eq(&sol.sha256) {
            eprintln!("hash is invalid");
            continue; // hash is invalid.
        }
        
        // Submit the hash.
        match app.submit_hash(&sol.sha256) {
            HashSubmittion::Accepted => {
                valid_solutions.push((leading_zero_bits, sol.clone()));
            },
            HashSubmittion::AlreadyExists => (),
        }

        if let Some(current_best) = &app.best {
            if leading_zero_bits > current_best.leading_zero_bit_length {
                let best = BestSolution {
                    student_number,
                    job_number: submit_request.job_n,
                    leading_zero_bit_length: leading_zero_bits,
                    hash: sol.sha256.clone(),
                    nounce: sol.nounce.clone(),
    
                };
                app.save_best(best.clone());
                app.best = Some(best);
            }
        } else {
            let best = BestSolution {
                student_number,
                job_number: submit_request.job_n,
                leading_zero_bit_length: leading_zero_bits,
                hash: sol.sha256.clone(),
                nounce: sol.nounce.clone(),
            };
            app.save_best(best.clone());
            app.best = Some(best);
        }
    }

    let mut submitter = (*app).submitter_from(&submit_request.student_number);

    // Check if the complete batch was returned.
    if submit_request.nounce_end < pending_job.nounce_end {
        // Found uncompleted portion. Added it to rejected jobs to be processed later.
        let number = submitter.next_job_number;
        submitter.next_job_number += 1;
        let nounce_start = submit_request.nounce_end;
        let nounce_end = pending_job.nounce_end;
        let size = nounce_end - nounce_start + 1;
        submitter.unfinished_jobs.push(StoredJob {
            number,
            size,
            nounce_start,
            nounce_end,
            quote_time: crate::util::get_time(),
        });
    }

    // add Solutions.
    submitter.accepted_shares_count += valid_solutions.len() as u64;
    for (leading, solution) in valid_solutions.into_iter() {
        submitter.save_solution(solution, leading);
    }

    // update machine info: thread hashrate.
    let reported_thread_hashrate = submit_request.thread_hashes_per_second;
    let mut machine = submitter.get_machine(&submit_request.name);
    machine.reported_thread_hashrate_history.push(reported_thread_hashrate);
    if machine.reported_thread_hashrate_history.len() > 100 {
        machine.reported_thread_hashrate_history.remove(0);
    }
    let sum: f64 = machine.reported_thread_hashrate_history.iter().sum();
    let len = machine.reported_thread_hashrate_history.len() as f64;
    machine.reported_thread_hashrate = sum / len;

    // update machine info: total hashrate.
    let reported_total_hashrate = submit_request.total_hashes_per_second;
    let mut machine = submitter.get_machine(&submit_request.name);
    machine.reported_total_hashrate_history.push(reported_total_hashrate);
    if machine.reported_total_hashrate_history.len() > 100 {
        machine.reported_total_hashrate_history.remove(0);
    }
    let sum: f64 = machine.reported_total_hashrate_history.iter().sum();
    let len = machine.reported_total_hashrate_history.len() as f64;
    machine.reported_total_hashrate = sum / len;

    // Recalculate next job size so that it is one minutes worth of work.
    let hashes_per_minute = machine.reported_thread_hashrate * 30.0;
    let next_job_size = max(hashes_per_minute.floor() as u64, 1_000_000);
    machine.calculated_job_size = next_job_size;

    submitter.save();
    HttpResponse::Ok().json(packets::SubmittionResponsePacket::Accepted)
}

fn max (a: u64, b: u64) -> u64 {
    if a > b { a } else { b }
}

fn sha245_to_string(sha256_buffer: &[u8]) -> String {
    let mut result = String::new();
    for byte in sha256_buffer {
        result += &format!("{:02x}", byte);
    }
    result
}

#[post("/status")]
pub async fn pool_status(data: AppData, status_request: Json<packets::PoolStatusRequestPacket>) -> impl Responder {
    let mut app = data.lock().unwrap();
    let submitter = app.submitter_from(&status_request.student_number);

    let user_total_hash_rate = submitter.user_hash_rate();
    let user_total_shares = submitter.accepted_shares_count as usize;
    let pool_total_shares = app.submitters.iter()
        .map(|(_student_number, submitter)| submitter.accepted_shares_count as usize)
        .sum();
    let next_job_sum: u64 = app.submitters.iter()
    .map(|(_student_number, submitter)| {
        submitter.next_job_number
    })
    .sum();
    let pending_job_sum: u64 =
        app.submitters.iter()
        .map(|(_sn, sub)| sub.pending_jobs.len() + sub.unfinished_jobs.len())
        .map(|v| v as u64)
        .sum();
    
    let completed_jobs = next_job_sum - pending_job_sum;
    
    let pool_best_zero_length = if let Some(current_best) = &app.best {
        current_best.leading_zero_bit_length
    } else {
        0
    };

    let packet = packets::PoolStatusResponsePacket {
        user_total_hash_rate,
        user_total_shares,
        pool_total_shares,
        pool_best_zero_length,
        completed_jobs,
    };
    HttpResponse::Ok().json(packet)
}


fn hash_to_sha256_buffer(hash: &String) -> Result<Vec<u8>, ()> {
    if hash.len() % 2 != 0 {
        return Err(());
    }
    let mut buffer = Vec::new();
    let mut it = hash.chars();
    while let (Some(upper), Some(lower)) = (it.next(), it.next()) {
        let mut upper = upper.to_digit(16).ok_or(())? as u8;
        upper <<= 4;
        let lower = lower.to_digit(16).ok_or(())? as u8;
        buffer.push(upper | lower);
    }
    Ok(buffer)
}

fn count_leading_zero_bits(buffer: &[u8]) -> u8 {
    let mut leading_zero_bits = 0;
    for byte in buffer {
        match byte {
            0 => {
                leading_zero_bits += 8;
            }
            0b0000_0001 => {
                leading_zero_bits += 7;
                break;
            }
            0b0000_0010 ..= 0b0000_0011 => {
                leading_zero_bits += 6;
                break;
            }
            0b0000_0100 ..= 0b0000_0111 => {
                leading_zero_bits += 5;
                break;
            }
            0b0000_1000 ..= 0b0000_1111 => {
                leading_zero_bits += 4;
                break;
            }
            0b0001_0000 ..= 0b0001_1111 => {
                leading_zero_bits += 3;
                break;
            }
            0b0010_0000 ..= 0b0011_1111 => {
                leading_zero_bits += 2;
                break;
            }
            0b0100_0000 ..= 0b0111_1111 => {
                leading_zero_bits += 1;
                break;
            }
            0b1000_0000 ..= 0b1111_1111 => {
                break;
            }
        }
    }
    leading_zero_bits
}
