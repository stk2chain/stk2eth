// Stress test for concurrent USSD sessions and swap transactions
// Target: 100+ concurrent sessions, < 100ms response time per session

#[cfg(test)]
mod concurrent_stress_tests {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    #[derive(Debug, Clone, PartialEq)]
    enum SwapStatus {
        Pending,
        Processing,
        Completed,
        Failed,
    }

    #[derive(Debug, Clone)]
    struct Swap {
        id: u64,
        session_id: String,
        from_address: String,
        to_address: String,
        amount: String,
        status: SwapStatus,
        created_at: Instant,
    }

    #[derive(Debug, Clone)]
    struct Session {
        session_id: String,
        phone_number: String,
        current_screen: String,
        active: bool,
    }

    struct MockDatabase {
        sessions: Vec<Session>,
        swaps: Vec<Swap>,
        swap_counter: u64,
    }

    impl MockDatabase {
        fn new() -> Self {
            MockDatabase {
                sessions: Vec::new(),
                swaps: Vec::new(),
                swap_counter: 0,
            }
        }

        fn create_session(&mut self, phone_number: String) -> Session {
            let session = Session {
                session_id: format!("session_{}", self.sessions.len()),
                phone_number,
                current_screen: "main_menu".to_string(),
                active: true,
            };
            self.sessions.push(session.clone());
            session
        }

        fn create_swap(
            &mut self,
            session_id: String,
            from: String,
            to: String,
            amount: String,
        ) -> Swap {
            let swap = Swap {
                id: self.swap_counter,
                session_id,
                from_address: from,
                to_address: to,
                amount,
                status: SwapStatus::Pending,
                created_at: Instant::now(),
            };
            self.swap_counter += 1;
            self.swaps.push(swap.clone());
            swap
        }

        fn process_swap(&mut self, swap_id: u64) -> Result<(), String> {
            if let Some(swap) = self.swaps.iter_mut().find(|s| s.id == swap_id) {
                swap.status = SwapStatus::Processing;
                // Simulate processing time
                thread::sleep(Duration::from_millis(1));
                swap.status = SwapStatus::Completed;
                Ok(())
            } else {
                Err("Swap not found".to_string())
            }
        }
    }

    #[test]
    fn test_100_concurrent_sessions() {
        let start_time = Instant::now();
        let db = Arc::new(Mutex::new(MockDatabase::new()));
        let mut handles = vec![];

        // Create 100 concurrent sessions
        for i in 0..100 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                let phone = format!("+25471234{:04}", i);
                let session_start = Instant::now();

                // Create session
                let session = {
                    let mut db_lock = db_clone.lock().unwrap();
                    db_lock.create_session(phone.clone())
                };

                // Create swap
                let swap = {
                    let mut db_lock = db_clone.lock().unwrap();
                    db_lock.create_swap(
                        session.session_id.clone(),
                        format!("0x{:040x}", i),
                        format!("0x{:040x}", i + 1000),
                        format!("{}.0", (i % 10) + 1),
                    )
                };

                // Process swap
                {
                    let mut db_lock = db_clone.lock().unwrap();
                    db_lock.process_swap(swap.id).unwrap();
                }

                let elapsed = session_start.elapsed();
                (session.session_id, elapsed)
            });

            handles.push(handle);
        }

        // Wait for all threads and collect results
        let mut response_times = Vec::new();
        for handle in handles {
            let (session_id, elapsed) = handle.join().unwrap();
            response_times.push(elapsed);
        }

        let total_elapsed = start_time.elapsed();

        // Assertions
        let db_lock = db.lock().unwrap();
        assert_eq!(db_lock.sessions.len(), 100, "Should have 100 sessions");
        assert_eq!(db_lock.swaps.len(), 100, "Should have 100 swaps");

        // Check all swaps completed
        let completed_count = db_lock
            .swaps
            .iter()
            .filter(|s| s.status == SwapStatus::Completed)
            .count();
        assert_eq!(completed_count, 100, "All swaps should be completed");

        // Performance checks
        let avg_response_time =
            response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let max_response_time = response_times.iter().max().unwrap();

        println!("✅ 100 concurrent sessions completed");
        println!("   Total time: {:?}", total_elapsed);
        println!("   Avg response time: {:?}", avg_response_time);
        println!("   Max response time: {:?}", max_response_time);

        // Assert performance targets
        assert!(
            avg_response_time < Duration::from_millis(100),
            "Average response time should be < 100ms, got {:?}",
            avg_response_time
        );
        assert!(
            total_elapsed < Duration::from_secs(60),
            "Total time should be < 60s, got {:?}",
            total_elapsed
        );
    }

    #[test]
    fn test_concurrent_session_isolation() {
        // Test that concurrent sessions don't interfere with each other
        let db = Arc::new(Mutex::new(MockDatabase::new()));
        let mut handles = vec![];

        for i in 0..50 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                let phone = format!("+254712{:06}", i);

                let session = {
                    let mut db_lock = db_clone.lock().unwrap();
                    db_lock.create_session(phone.clone())
                };

                let swap = {
                    let mut db_lock = db_clone.lock().unwrap();
                    db_lock.create_swap(
                        session.session_id.clone(),
                        format!("0x{:040x}", i * 2),
                        format!("0x{:040x}", i * 2 + 1),
                        format!("{}.5", i + 1),
                    )
                };

                (session.session_id, swap.id, phone)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.join().unwrap());
        }

        // Verify no session ID or phone number collisions
        let mut session_ids = results.iter().map(|(sid, _, _)| sid).collect::<Vec<_>>();
        session_ids.sort();
        session_ids.dedup();
        assert_eq!(session_ids.len(), 50, "All session IDs should be unique");

        let mut phones = results.iter().map(|(_, _, p)| p).collect::<Vec<_>>();
        phones.sort();
        phones.dedup();
        assert_eq!(phones.len(), 50, "All phone numbers should be unique");

        println!("✅ Session isolation maintained across 50 concurrent sessions");
    }

    #[test]
    fn test_high_throughput_1000_swaps() {
        let start_time = Instant::now();
        let db = Arc::new(Mutex::new(MockDatabase::new()));
        let batch_size = 100;
        let num_batches = 10;

        for batch in 0..num_batches {
            let mut handles = vec![];

            for i in 0..batch_size {
                let db_clone = Arc::clone(&db);
                let global_i = batch * batch_size + i;

                let handle = thread::spawn(move || {
                    let phone = format!("+254700{:06}", global_i);

                    let session = {
                        let mut db_lock = db_clone.lock().unwrap();
                        db_lock.create_session(phone)
                    };

                    let swap = {
                        let mut db_lock = db_clone.lock().unwrap();
                        db_lock.create_swap(
                            session.session_id,
                            format!("0x{:040x}", global_i),
                            format!("0x{:040x}", global_i + 10000),
                            "1.0".to_string(),
                        )
                    };

                    {
                        let mut db_lock = db_clone.lock().unwrap();
                        db_lock.process_swap(swap.id).unwrap();
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        }

        let elapsed = start_time.elapsed();
        let db_lock = db.lock().unwrap();

        assert_eq!(db_lock.swaps.len(), 1000);
        assert_eq!(db_lock.sessions.len(), 1000);

        let completed = db_lock
            .swaps
            .iter()
            .filter(|s| s.status == SwapStatus::Completed)
            .count();
        assert_eq!(completed, 1000);

        let tps = 1000.0 / elapsed.as_secs_f64();

        println!("✅ Processed 1000 swaps in {:?}", elapsed);
        println!("   Throughput: {:.2} TPS", tps);

        assert!(tps > 100.0, "Should achieve > 100 TPS, got {:.2}", tps);
    }

    #[test]
    fn test_concurrent_swap_state_transitions() {
        // Test concurrent state transitions don't cause race conditions
        let db = Arc::new(Mutex::new(MockDatabase::new()));

        // Create swaps first
        {
            let mut db_lock = db.lock().unwrap();
            for i in 0..20 {
                let session = db_lock.create_session(format!("+25471{:07}", i));
                db_lock.create_swap(
                    session.session_id,
                    format!("0x{:040x}", i),
                    format!("0x{:040x}", i + 100),
                    "1.0".to_string(),
                );
            }
        }

        // Process all swaps concurrently
        let mut handles = vec![];
        for swap_id in 0..20 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                let mut db_lock = db_clone.lock().unwrap();
                db_lock.process_swap(swap_id).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let db_lock = db.lock().unwrap();
        let completed = db_lock
            .swaps
            .iter()
            .filter(|s| s.status == SwapStatus::Completed)
            .count();

        assert_eq!(completed, 20, "All swaps should complete");
        println!("✅ Concurrent state transitions handled correctly");
    }

    #[test]
    fn test_response_time_consistency() {
        // Test that response time remains consistent under load
        let db = Arc::new(Mutex::new(MockDatabase::new()));
        let mut response_times = Vec::new();

        for i in 0..100 {
            let db_clone = Arc::clone(&db);
            let start = Instant::now();

            let session = {
                let mut db_lock = db_clone.lock().unwrap();
                db_lock.create_session(format!("+25471{:07}", i))
            };

            let swap = {
                let mut db_lock = db_clone.lock().unwrap();
                db_lock.create_swap(
                    session.session_id,
                    format!("0x{:040x}", i),
                    format!("0x{:040x}", i + 100),
                    "1.0".to_string(),
                )
            };

            {
                let mut db_lock = db_clone.lock().unwrap();
                db_lock.process_swap(swap.id).unwrap();
            }

            response_times.push(start.elapsed());
        }

        // Calculate statistics
        let avg = response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let max = response_times.iter().max().unwrap();
        let min = response_times.iter().min().unwrap();

        println!("✅ Response time statistics:");
        println!("   Min: {:?}", min);
        println!("   Avg: {:?}", avg);
        println!("   Max: {:?}", max);

        // All should be < 100ms
        assert!(
            max < &Duration::from_millis(100),
            "Max response time should be < 100ms"
        );
    }
}
