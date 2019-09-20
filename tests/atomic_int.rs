#![deny(warnings, rust_2018_idioms)]

macro_rules! test_int {
    ($name:ident, $int:ty, $atomic:ty) => {
        mod $name {
            use loom::sync::atomic::*;
            use std::sync::atomic::Ordering::SeqCst;

            const NUM_A: u64 = 11641914933775430211;
            const NUM_B: u64 = 13209405719799650717;

            #[test]
            fn xor() {
                loom::model(|| {
                    let a: $int = NUM_A as $int;
                    let b: $int = NUM_B as $int;

                    let atomic = <$atomic>::new(a);
                    let prev = atomic.fetch_xor(b, SeqCst);

                    assert_eq!(a, prev);
                    assert_eq!(a ^ b, atomic.load(SeqCst));
                });
            }

            #[test]
            fn compare_exchange() {
                loom::model(|| {
                    let a: $int = NUM_A as $int;
                    let b: $int = NUM_B as $int;

                    let atomic = <$atomic>::new(a);
                    assert_eq!(Err(a), atomic.compare_exchange(b, a, SeqCst, SeqCst));
                    assert_eq!(Ok(a), atomic.compare_exchange(a, b, SeqCst, SeqCst));

                    assert_eq!(b, atomic.load(SeqCst));
                });
            }

            #[test]
            fn compare_exchange_weak() {
                loom::model(|| {
                    let a: $int = NUM_A as $int;
                    let b: $int = NUM_B as $int;

                    let atomic = <$atomic>::new(a);
                    assert_eq!(Err(a), atomic.compare_exchange_weak(b, a, SeqCst, SeqCst));
                    assert_eq!(Ok(a), atomic.compare_exchange_weak(a, b, SeqCst, SeqCst));

                    assert_eq!(b, atomic.load(SeqCst));
                });
            }
        }
    };
}

test_int!(atomic_u8, u8, AtomicU8);
test_int!(atomic_u16, u16, AtomicU16);
test_int!(atomic_u32, u32, AtomicU32);
test_int!(atomic_usize, usize, AtomicUsize);
