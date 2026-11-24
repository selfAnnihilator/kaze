use super::*;
use rodio::source::SineWave;
use rodio::Source;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

/// A sink whose "audio thread" only pulls samples while `awake` is set,
/// like the real device after Kaze suspends it.
fn paused_sink_with_sleeping_device() -> (Sink, Arc<AtomicBool>, Arc<AtomicBool>) {
    let (sink, mut output) = Sink::new_idle();
    sink.append(SineWave::new(440.0).take_duration(Duration::from_secs(60)));
    sink.pause();
    let awake = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let (a, s) = (awake.clone(), stop.clone());
    std::thread::spawn(move || {
        while !s.load(Ordering::Relaxed) {
            if a.load(Ordering::Relaxed) {
                for _ in 0..512 {
                    output.next();
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    (sink, awake, stop)
}

#[test]
fn a_seek_while_paused_wakes_the_device_and_completes() {
    let (sink, awake, stop) = paused_sink_with_sleeping_device();
    let (done, finished) = mpsc::channel();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        let set = |a: Activity| {
            log.lock().unwrap().push(a);
            awake.store(a == Activity::Playing, Ordering::Relaxed);
        };
        let result = seek_awake(true, set, || sink.try_seek(Duration::from_secs(10)));
        let _ = done.send(result.is_ok());
    });
    let ok = finished.recv_timeout(Duration::from_secs(5)).expect("the seek must not hang while paused");
    stop.store(true, Ordering::Relaxed);
    assert!(ok);
    assert_eq!(*seen.lock().unwrap(), [Activity::Playing, Activity::Idle], "woken for the seek, then idle again");
}

#[test]
fn a_seek_while_playing_leaves_the_device_alone() {
    let calls = std::cell::RefCell::new(Vec::new());
    let value = seek_awake(false, |a| calls.borrow_mut().push(a), || 7);
    assert_eq!(value, 7);
    assert!(calls.borrow().is_empty());
}

#[test]
fn without_waking_the_device_the_seek_hangs() {
    // The bug this guards against: rodio waits for an audio thread that is asleep.
    let (sink, _awake, stop) = paused_sink_with_sleeping_device();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sink.try_seek(Duration::from_secs(10));
        let _ = done.send(());
    });
    assert!(finished.recv_timeout(Duration::from_millis(800)).is_err(), "rodio's seek blocks while the device sleeps");
    stop.store(true, Ordering::Relaxed);
}
