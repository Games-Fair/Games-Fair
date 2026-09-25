// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Demand-driven game loops on Day's native, window-scoped display scheduler.

use day_core::frame::{FrameClock, FrameHandle};
use day_pieces::prelude::*;
use std::{cell::RefCell, ops::ControlFlow, rc::Rc, time::Duration};

/// A zero-size piece owning a display subscription for its mounting scope.
///
/// `needed` tracks signals/triggers that change when input starts work. It is also checked
/// before and after each tick, so a completed animation sleeps immediately and a later move
/// wakes it without rebuilding the piece. Include pending cues, AI turns and result delays
/// in the predicate, not just visible movement. Keep it free of mutations.
///
/// `tick` receives real elapsed time, including zero on the first/resumed frame. Games choose
/// their own simulation cap; clocks and countdowns should use the unmodified duration.
/// Scope disposal cancels the subscription. Day handles backgrounding and window closure.
pub fn clock(
    needed: impl Fn() -> bool + 'static,
    tick: impl FnMut(Duration) + 'static,
) -> impl Piece {
    let callbacks = RefCell::new(Some((needed, tick)));
    piece_fn(move |cx| {
        if let Some((needed, tick)) = callbacks.borrow_mut().take() {
            subscribe(needed, tick);
        }
        label("").frame(0.0, 0.0).build(cx)
    })
}

fn subscribe(needed: impl Fn() -> bool + 'static, tick: impl FnMut(Duration) + 'static) {
    let clock = FrameClock::current();
    let needed = Rc::new(needed);
    let handle = Rc::new(RefCell::new(None::<FrameHandle>));
    let tick = RefCell::new(Some(tick));
    let cleanup = handle.clone();
    Scope::current().on_cleanup(move || {
        // Release outside the slot borrow: dropping a callback may release game state.
        let handle = cleanup.borrow_mut().take();
        drop(handle);
    });
    Effect::new(move || {
        if needed() {
            if let Some(handle) = handle.borrow().as_ref() {
                handle.resume();
                return;
            }
            let needed = needed.clone();
            let mut tick = tick
                .borrow_mut()
                .take()
                .expect("one subscription per scope");
            let subscription = clock.subscribe(move |frame| {
                if !day_reactive::untrack(|| needed()) {
                    return ControlFlow::Break(());
                }
                tick(frame.delta);
                if day_reactive::untrack(|| needed()) {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            });
            *handle.borrow_mut() = Some(subscription);
        } else if let Some(handle) = handle.borrow().as_ref() {
            handle.pause();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use day_spec::{FrameCallback, FrameStamp};
    use std::{cell::Cell, collections::VecDeque};

    #[test]
    fn idle_wake_settle_pause_resume_and_scope_cleanup() {
        let pending = Rc::new(RefCell::new(
            VecDeque::<(Rc<Cell<bool>>, FrameCallback)>::new(),
        ));
        let requests = Rc::new(Cell::new(0));
        let (queue, count) = (pending.clone(), requests.clone());
        day_core::install_frame_requester(move |_, callback| {
            count.set(count.get() + 1);
            let cancelled = Rc::new(Cell::new(false));
            queue.borrow_mut().push_back((cancelled.clone(), callback));
            Box::new(move || cancelled.set(true))
        });
        let fire = |timestamp| {
            loop {
                let (cancelled, callback) =
                    pending.borrow_mut().pop_front().expect("pending frame");
                if !cancelled.get() {
                    callback(FrameStamp::new(timestamp));
                    break;
                }
            }
        };
        let scope = Scope::child();
        let moving = Rc::new(Cell::new(false));
        let deltas = Rc::new(RefCell::new(Vec::new()));
        let (wake, paused) = day_core::with_page_in(
            Default::default(),
            None,
            day_spec::ToolbarColumn::Window,
            Default::default(),
            || {
                scope.enter(|| {
                    let wake = Trigger::new();
                    let paused = Signal::new(false);
                    let (needed, step, seen) = (moving.clone(), moving.clone(), deltas.clone());
                    subscribe(
                        move || {
                            wake.track();
                            !paused.get() && needed.get()
                        },
                        move |dt| {
                            seen.borrow_mut().push(dt);
                            if dt >= Duration::from_millis(200) {
                                // No notification: post-tick demand must stop the native source.
                                step.set(false);
                            }
                        },
                    );
                    (wake, paused)
                })
            },
        );
        assert_eq!(requests.get(), 0, "idle mount must not request frames");
        moving.set(true);
        wake.notify();
        day_reactive::flush_now();
        fire(1.0);
        fire(1.25);
        assert_eq!(
            *deltas.borrow(),
            vec![Duration::ZERO, Duration::from_millis(250)]
        );
        assert!(
            pending.borrow().is_empty(),
            "settled animation must stop requesting"
        );

        moving.set(true);
        wake.notify();
        day_reactive::flush_now();
        fire(20.0);
        assert_eq!(*deltas.borrow().last().unwrap(), Duration::ZERO);
        paused.set(true);
        day_reactive::flush_now();
        assert!(pending.borrow().back().unwrap().0.get());
        paused.set(false);
        day_reactive::flush_now();
        fire(40.0);
        assert_eq!(*deltas.borrow().last().unwrap(), Duration::ZERO);
        scope.dispose();
        let calls = deltas.borrow().len();
        for (_, callback) in pending.borrow_mut().drain(..) {
            callback(FrameStamp::new(41.0)); // Native delivery already queued at cancellation.
        }
        assert_eq!(deltas.borrow().len(), calls);
        assert_eq!(day_core::frame_consumer_count(), 0);
    }
}
