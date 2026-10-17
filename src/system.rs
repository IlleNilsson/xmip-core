//! xmip-core's calls to the operating system, each behind a safe function:
//! on Windows the kernel event and the high-resolution waitable timer the
//! timed wait is made of (`wake`; the owner, 2026-10-16), and, for tests,
//! the processor time a thread and the process have used (`processor_time`)
//! — cycles on Windows, nanoseconds on Unix.
//!
//! The one file in this crate that may hold unsafe code (ADR-0050,
//! amendment 2026-10-17): these are C interfaces and are reached no other
//! way. Every block says where its pointers come from, how long they live
//! and who closes the handles.
#![allow(unsafe_code)]

#[cfg(windows)]
pub(crate) use windows::{Event, Timer, either};
#[cfg(all(windows, feature = "test-support"))]
pub(crate) use windows::{process_cycles, thread_cycles};

#[cfg(all(unix, feature = "test-support"))]
pub(crate) use unix::{process_cpu_nanos, thread_cpu_nanos};

#[cfg(windows)]
mod windows {
    use std::ptr::null;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CancelWaitableTimer, CreateEventW,
        CreateWaitableTimerExW, INFINITE, SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS,
        WaitForMultipleObjects, WaitForSingleObject,
    };

    /// An auto-reset kernel event: one thread waits on it, any sets it.
    pub(crate) struct Event(HANDLE);

    // SAFETY: a kernel handle names the object in the whole process;
    // setting and waiting on it from any thread is what it is for.
    unsafe impl Send for Event {}
    // SAFETY: as for Send; nothing is mutated through a shared reference.
    unsafe impl Sync for Event {}

    impl Event {
        /// A new event, not set.
        ///
        /// # Panics
        ///
        /// Where Windows makes no event: the process is out of handles.
        pub(crate) fn new() -> Self {
            // SAFETY: no attributes and no name, so no pointer is read. The
            // handle is this value's and closed in its Drop.
            let handle = unsafe { CreateEventW(null(), 0, 0, null()) };
            assert!(
                !handle.is_null(),
                "a kernel event for a waiting thread: {}",
                std::io::Error::last_os_error()
            );
            Self(handle)
        }

        pub(crate) fn set(&self) {
            // SAFETY: the handle is open while this value lives.
            unsafe { SetEvent(self.0) };
        }

        pub(crate) fn wait(&self) {
            // SAFETY: the handle is open while this value lives.
            unsafe { WaitForSingleObject(self.0, INFINITE) };
        }

        /// Wait at most `left`, counted in whole milliseconds rounded up;
        /// whether it was set.
        pub(crate) fn wait_millis(&self, left: Duration) -> bool {
            let millis = u32::try_from(left.as_nanos().div_ceil(1_000_000))
                .unwrap_or(INFINITE - 1)
                .min(INFINITE - 1);
            // SAFETY: the handle is open while this value lives.
            unsafe { WaitForSingleObject(self.0, millis) == WAIT_OBJECT_0 }
        }
    }

    impl Drop for Event {
        fn drop(&mut self) {
            // SAFETY: made in Event::new, closed once, here.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// A high-resolution, auto-reset waitable timer; none where Windows
    /// could not make one (before Windows 10, version 1803).
    pub(crate) struct Timer(HANDLE);

    impl Timer {
        pub(crate) fn new() -> Self {
            // SAFETY: no attributes and no name, so no pointer is read. The
            // handle, where there is one, is this value's, closed in Drop.
            Self(unsafe {
                CreateWaitableTimerExW(
                    null(),
                    null(),
                    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                    TIMER_ALL_ACCESS,
                )
            })
        }

        /// Due in `left`, at least 100 ns from now; whether it was armed.
        pub(crate) fn arm(&self, left: Duration) -> bool {
            if self.0.is_null() {
                return false;
            }
            // Negative: relative, in 100-nanosecond intervals.
            let due = -i64::try_from(left.as_nanos() / 100)
                .unwrap_or(i64::MAX)
                .max(1);
            // SAFETY: the handle is open while this value lives; `due`
            // lives across the call, which only reads it; no routine.
            unsafe { SetWaitableTimer(self.0, &raw const due, 0, None, null(), 0) != 0 }
        }

        pub(crate) fn cancel(&self) {
            if !self.0.is_null() {
                // SAFETY: the handle is open while this value lives.
                unsafe { CancelWaitableTimer(self.0) };
            }
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: made in Timer::new, closed once, here.
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// Wait until `event` is set or armed `timer` is due; whether it was
    /// the event. A failed wait says it was not.
    pub(crate) fn either(event: &Event, timer: &Timer) -> bool {
        let both = [event.0, timer.0];
        // SAFETY: `both` lives across the call, which reads its two
        // handles, open while `event` and `timer` live.
        let woke = unsafe { WaitForMultipleObjects(2, both.as_ptr(), 0, INFINITE) };
        woke == WAIT_OBJECT_0
    }

    /// Processor cycles this thread has used; none where Windows says no.
    #[cfg(feature = "test-support")]
    pub(crate) fn thread_cycles() -> Option<u64> {
        use windows_sys::Win32::System::Threading::GetCurrentThread;
        use windows_sys::Win32::System::WindowsProgramming::QueryThreadCycleTime;
        let mut cycles = 0_u64;
        // SAFETY: the current thread's pseudo handle needs no closing;
        // `cycles` lives across the call, which writes it once.
        let done = unsafe { QueryThreadCycleTime(GetCurrentThread(), &raw mut cycles) };
        (done != 0).then_some(cycles)
    }

    /// Processor cycles the process has used; none where Windows says no.
    #[cfg(feature = "test-support")]
    pub(crate) fn process_cycles() -> Option<u64> {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        use windows_sys::Win32::System::WindowsProgramming::QueryProcessCycleTime;
        let mut cycles = 0_u64;
        // SAFETY: the current process's pseudo handle needs no closing;
        // `cycles` lives across the call, which writes it once.
        let done = unsafe { QueryProcessCycleTime(GetCurrentProcess(), &raw mut cycles) };
        (done != 0).then_some(cycles)
    }
}

#[cfg(all(unix, feature = "test-support"))]
mod unix {
    /// Nanoseconds of processor time this thread has used.
    pub(crate) fn thread_cpu_nanos() -> Option<u64> {
        cpu_nanos(libc::CLOCK_THREAD_CPUTIME_ID)
    }

    /// Nanoseconds of processor time the process has used.
    pub(crate) fn process_cpu_nanos() -> Option<u64> {
        cpu_nanos(libc::CLOCK_PROCESS_CPUTIME_ID)
    }

    fn cpu_nanos(clock: libc::clockid_t) -> Option<u64> {
        let mut now = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `now` lives across the call, which writes it once.
        let done = unsafe { libc::clock_gettime(clock, &raw mut now) };
        if done != 0 {
            return None;
        }
        let seconds = u64::try_from(now.tv_sec).ok()?;
        let nanos = u64::try_from(now.tv_nsec).ok()?;
        seconds.checked_mul(1_000_000_000)?.checked_add(nanos)
    }
}
