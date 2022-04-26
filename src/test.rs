use crate::RateLimiter;

#[test]
fn test() {
    let slots = 10;
    let limit = slots * 10;
    let rate_limiter = RateLimiter::<u32>::new(limit, slots);

    let check_attempts = |key: u32, n: usize, expect: bool| {
        for _ in 0..n {
            assert_eq!(
                rate_limiter.check_limited(key.clone()),
                expect,
                "Check: limited[{}] == {}",
                key,
                expect,
            );
        }
    };

    // Check limit reached in 1 slot for key 0
    check_attempts(0, limit, false);
    check_attempts(0, slots * 2, true);
    // Check limit not reached for key 1
    check_attempts(1, limit, false);
    // Reset
    for _ in 0..limit {
        rate_limiter.rotate_slots();
    }
    for _ in 0..3 {
        // Run 2 full cycles where each slot is maxed out with even distribution for key 0
        for _ in 0..(slots * 2) {
            check_attempts(0, limit / slots, false);
            // Check limit not reached for key 1
            check_attempts(1, limit / slots, false);
            rate_limiter.rotate_slots();
        }
        // Fill current slot for key 0
        check_attempts(0, slots, false);
        // Check limit reached for key 0, additional counts should not be tracked
        check_attempts(0, slots + 3, true);
        rate_limiter.rotate_slots();
        // Check limit still reached for key 0
        check_attempts(0, slots, true);
        rate_limiter.rotate_slots();
        rate_limiter.rotate_slots();
        // Check limit no longer reached after two rotations
        check_attempts(0, slots, false);
        rate_limiter.rotate_slots();
    }
}
