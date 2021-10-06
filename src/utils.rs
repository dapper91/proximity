use std::future::Future;
use std::iter::IntoIterator;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(PartialEq)]
pub enum Strategy {
    Any,
    All,
}

pub enum FutureState<F>
where
    F: Future,
    F::Output: Unpin,
{
    Ready(F::Output),
    Pending(Pin<Box<F>>),
}

pub struct WaitFor<F>
where
    F: Future,
    F::Output: Unpin,
{
    strategy: Strategy,
    future_states: Vec<FutureState<F>>,
}

impl<F> WaitFor<F>
where
    F: Future,
    F::Output: Unpin,
{
    pub fn cease(self) -> Vec<FutureState<F>> {
        self.future_states
    }
}

impl<F> Future for WaitFor<F>
where
    F: Future,
    F::Output: Unpin,
{
    type Output = Vec<FutureState<F>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ready_cnt = 0;

        for future_state in self.future_states.iter_mut() {
            match future_state {
                FutureState::Ready(_) => ready_cnt += 1,
                FutureState::Pending(fut) => match fut.as_mut().poll(cx) {
                    Poll::Pending => {}
                    Poll::Ready(res) => {
                        *future_state = FutureState::Ready(res);
                        ready_cnt += 1;
                    }
                },
            }
        }

        if self.strategy == Strategy::Any && ready_cnt > 0
            || self.strategy == Strategy::All && ready_cnt == self.future_states.len()
        {
            Poll::Ready(mem::replace(&mut self.future_states, vec![]))
        } else {
            Poll::Pending
        }
    }
}

pub trait AsFutureStates<F>
where
    F: Future,
    F::Output: Unpin,
{
    fn as_futures_states(self) -> Vec<FutureState<F>>;
}

impl<F> AsFutureStates<F> for Vec<F>
where
    F: Future,
    F::Output: Unpin,
{
    fn as_futures_states(self) -> Vec<FutureState<F>> {
        self.into_iter()
            .map(|fut| FutureState::Pending(Box::pin(fut)))
            .collect()
    }
}

impl<F> AsFutureStates<F> for Vec<FutureState<F>>
where
    F: Future,
    F::Output: Unpin,
{
    fn as_futures_states(self) -> Vec<FutureState<F>> {
        self.into_iter().collect()
    }
}

pub fn wait_for<I, F>(futures: I, strategy: Strategy) -> WaitFor<F>
where
    I: AsFutureStates<F>,
    F: Future,
    F::Output: Unpin,
{
    return WaitFor {
        future_states: futures.as_futures_states(),
        strategy,
    };
}

pub fn wait_for_all<I, F>(futures: I) -> WaitFor<F>
where
    I: AsFutureStates<F>,
    F: Future,
    F::Output: Unpin,
{
    return wait_for(futures, Strategy::All);
}

pub fn wait_for_any<I, F>(futures: I) -> WaitFor<F>
where
    I: AsFutureStates<F>,
    F: Future,
    F::Output: Unpin,
{
    return wait_for(futures, Strategy::Any);
}
