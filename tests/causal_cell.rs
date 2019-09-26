#![deny(warnings, rust_2018_idioms)]

use loom;

use loom::sync::atomic::AtomicUsize;
use loom::sync::CausalCell;
use loom::thread;

use std::sync::atomic::Ordering::{Acquire, Release, SeqCst};
use std::sync::Arc;

#[test]
fn atomic_causality_success() {
    struct Chan {
        data: CausalCell<usize>,
        guard: AtomicUsize,
    }

    impl Chan {
        fn set(&self) {
            unsafe {
                self.data.with_mut(|v| {
                    *v += 123;
                });
            }

            self.guard.store(1, Release);
        }

        fn get(&self) {
            if 0 == self.guard.load(Acquire) {
                return;
            }

            unsafe {
                self.data.with(|v| {
                    assert_eq!(*v, 123);
                });
            }
        }
    }

    loom::model(|| {
        let chan = Arc::new(Chan {
            data: CausalCell::new(0),
            guard: AtomicUsize::new(0),
        });

        let th = {
            let chan = chan.clone();
            thread::spawn(move || {
                chan.set();
            })
        };

        // Try getting before joining
        chan.get();

        th.join().unwrap();

        chan.get();
    });
}

#[test]
#[should_panic]
fn atomic_causality_fail() {
    struct Chan {
        data: CausalCell<usize>,
        guard: AtomicUsize,
    }

    impl Chan {
        fn set(&self) {
            unsafe {
                self.data.with_mut(|v| {
                    *v += 123;
                });
            }

            self.guard.store(1, Release);
        }

        fn get(&self) {
            unsafe {
                self.data.with(|v| {
                    assert_eq!(*v, 123);
                });
            }
        }
    }

    loom::model(|| {
        let chan = Arc::new(Chan {
            data: CausalCell::new(0),
            guard: AtomicUsize::new(0),
        });

        let th = {
            let chan = chan.clone();
            thread::spawn(move || chan.set())
        };

        // Try getting before joining
        chan.get();

        th.join().unwrap();

        chan.get();
    });
}

#[derive(Clone)]
struct Data(Arc<CausalCell<usize>>);

impl Data {
    fn new(v: usize) -> Self {
        Data(Arc::new(CausalCell::new(v)))
    }

    fn get(&self) -> usize {
        self.0.with(|v| unsafe { *v })
    }

    fn inc(&self) -> usize {
        self.0.with_mut(|v| unsafe {
            *v += 1;
            *v
        })
    }
}

#[test]
#[should_panic]
fn causal_cell_race_mut_mut_1() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();

        let th1 = thread::spawn(move || x.inc());
        y.inc();

        th1.join().unwrap();

        assert_eq!(4, y.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_mut_2() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();
        let z = x.clone();

        let th1 = thread::spawn(move || x.inc());
        let th2 = thread::spawn(move || y.inc());

        th1.join().unwrap();
        th2.join().unwrap();

        assert_eq!(4, z.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_immut_1() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();

        let th1 = thread::spawn(move || assert_eq!(2, x.inc()));
        y.get();

        th1.join().unwrap();

        assert_eq!(3, y.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_immut_2() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();

        let th1 = thread::spawn(move || x.get());
        assert_eq!(2, y.inc());

        th1.join().unwrap();

        assert_eq!(3, y.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_immut_3() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();
        let z = x.clone();

        let th1 = thread::spawn(move || assert_eq!(2, x.inc()));
        let th2 = thread::spawn(move || y.get());

        th1.join().unwrap();
        th2.join().unwrap();

        assert_eq!(3, z.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_immut_4() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();
        let z = x.clone();

        let th1 = thread::spawn(move || x.get());
        let th2 = thread::spawn(move || assert_eq!(2, y.inc()));

        th1.join().unwrap();
        th2.join().unwrap();

        assert_eq!(3, z.inc());
    });
}

#[test]
#[should_panic]
fn causal_cell_race_mut_immut_5() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();
        let z = x.clone();

        let th1 = thread::spawn(move || x.get());
        let th2 = thread::spawn(move || {
            assert_eq!(1, y.get());
            assert_eq!(2, y.inc());
        });

        th1.join().unwrap();
        th2.join().unwrap();

        assert_eq!(3, z.inc());
    });
}

#[test]
fn causal_cell_ok_1() {
    loom::model(|| {
        let x = Data::new(1);

        assert_eq!(2, x.inc());

        let th1 = thread::spawn(move || {
            assert_eq!(3, x.inc());
            x
        });

        let x = th1.join().unwrap();

        assert_eq!(4, x.inc());
    });
}

#[test]
fn causal_cell_ok_2() {
    loom::model(|| {
        let x = Data::new(1);

        assert_eq!(1, x.get());
        assert_eq!(2, x.inc());

        let th1 = thread::spawn(move || {
            assert_eq!(2, x.get());
            assert_eq!(3, x.inc());
            x
        });

        let x = th1.join().unwrap();

        assert_eq!(3, x.get());
        assert_eq!(4, x.inc());
    });
}

#[test]
fn causal_cell_ok_3() {
    loom::model(|| {
        let x = Data::new(1);
        let y = x.clone();

        let th1 = thread::spawn(move || {
            assert_eq!(1, x.get());

            let z = x.clone();
            let th2 = thread::spawn(move || {
                assert_eq!(1, z.get());
            });

            assert_eq!(1, x.get());
            th2.join().unwrap();
        });

        assert_eq!(1, y.get());

        th1.join().unwrap();

        assert_eq!(2, y.inc());
    });
}

// The test shows an algorithm that panics if defer is not used.
#[test]
#[should_panic]
fn should_defer() {
    use std::mem::MaybeUninit;

    loom::model(|| {
        let s1 = Arc::new((CausalCell::new(MaybeUninit::new(0)), AtomicUsize::new(0)));
        let s2 = s1.clone();

        let th = thread::spawn(move || {
            s2.1.store(1, SeqCst);
            s2.0.with_mut(|ptr| unsafe { *(*ptr).as_mut_ptr() = 1 });
        });

        let mem = s1.0.with(|ptr| unsafe { *ptr });

        if 0 == s1.1.load(SeqCst) {
            assert_eq!(unsafe { *mem.as_ptr() }, 0);
        }

        th.join().unwrap();
    });
}

// Works w/ defer
#[test]
fn defer_success() {
    use std::mem::MaybeUninit;

    loom::model(|| {
        let s1 = Arc::new((CausalCell::new(MaybeUninit::new(0)), AtomicUsize::new(0)));
        let s2 = s1.clone();

        let th = thread::spawn(move || {
            s2.1.store(1, SeqCst);
            s2.0.with_mut(|ptr| unsafe { *(*ptr).as_mut_ptr() = 1 });
        });

        let (mem, check) = s1.0.with_deferred(|ptr| unsafe { *ptr });

        if 0 == s1.1.load(SeqCst) {
            assert_eq!(unsafe { *mem.as_ptr() }, 0);
            check.check();
        }

        th.join().unwrap();
    });
}

// Incorrect call to defer panics
#[test]
#[should_panic]
fn defer_fail() {
    use std::mem::MaybeUninit;

    loom::model(|| {
        let s1 = Arc::new((CausalCell::new(MaybeUninit::new(0)), AtomicUsize::new(0)));
        let s2 = s1.clone();

        let th = thread::spawn(move || {
            s2.1.store(1, SeqCst);
            s2.0.with_mut(|ptr| unsafe { *(*ptr).as_mut_ptr() = 1 });
        });

        let (mem, check) = s1.0.with_deferred(|ptr| unsafe { *ptr });

        if 0 == s1.1.load(SeqCst) {
            assert_eq!(unsafe { *mem.as_ptr() }, 0);
        } else {
            check.check();
        }

        th.join().unwrap();
    });
}

#[test]
fn batch_defer_success() {
    use loom::sync::CausalCheck;
    use std::mem::MaybeUninit;

    loom::model(|| {
        let state = (0..2)
            .map(|_| (CausalCell::new(MaybeUninit::new(0)), AtomicUsize::new(0)))
            .collect::<Vec<_>>();

        let s1 = Arc::new(state);
        let s2 = s1.clone();

        let th = thread::spawn(move || {
            s2[0].1.store(1, SeqCst);
            s2[0].0.with_mut(|ptr| unsafe { *(*ptr).as_mut_ptr() = 1 });
        });

        let mut check = CausalCheck::default();

        let (mem0, c) = s1[0].0.with_deferred(|ptr| unsafe { *ptr });
        check.join(c);

        let (mem1, c) = s1[0].0.with_deferred(|ptr| unsafe { *ptr });
        check.join(c);

        if 0 != s1[0].1.load(SeqCst) {
            return;
        }

        check.check();

        assert_eq!(unsafe { *mem0.as_ptr() }, 0);
        assert_eq!(unsafe { *mem1.as_ptr() }, 0);

        th.join().unwrap();
    });
}

#[test]
#[should_panic]
fn batch_defer_fail() {
    use loom::sync::CausalCheck;
    use std::mem::MaybeUninit;

    loom::model(|| {
        let state = (0..2)
            .map(|_| (CausalCell::new(MaybeUninit::new(0)), AtomicUsize::new(0)))
            .collect::<Vec<_>>();

        let s1 = Arc::new(state);
        let s2 = s1.clone();

        let th = thread::spawn(move || {
            s2[0].1.store(1, SeqCst);
            s2[0].0.with_mut(|ptr| unsafe { *(*ptr).as_mut_ptr() = 1 });
        });

        let mut check = CausalCheck::default();

        let (mem0, c) = s1[0].0.with_deferred(|ptr| unsafe { *ptr });
        check.join(c);

        let (mem1, c) = s1[0].0.with_deferred(|ptr| unsafe { *ptr });
        check.join(c);

        check.check();

        assert_eq!(unsafe { *mem0.as_ptr() }, 0);
        assert_eq!(unsafe { *mem1.as_ptr() }, 0);

        th.join().unwrap();
    });
}
