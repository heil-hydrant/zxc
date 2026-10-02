use std::fmt::Display;

use tracing::{Instrument, Level, span};

// Trait like AsyncIterator
pub trait AsyncStep {
    type Error;

    async fn next(self) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn is_ended(&self) -> bool;
}

pub async fn async_run<T>(mut state: T) -> Result<T, T::Error>
where
    T: AsyncStep + Display,
{
    loop {
        let span = span(&state);
        state = state.next().instrument(span).await?;
        if state.is_ended() {
            return Ok(state);
        }
    }
}

fn span<T>(state: T) -> tracing::Span
where
    T: Display,
{
    let span = span!(Level::TRACE, "", "{}", state.to_string());
    let _ = span.enter();
    span
}
