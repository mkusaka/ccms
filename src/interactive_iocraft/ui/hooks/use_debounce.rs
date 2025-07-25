//! Debounce hook for delayed value updates

use iocraft::prelude::*;
use std::time::{Duration, Instant};

/// Hook that debounces value changes by the specified duration
pub fn use_debounce<T: Clone + Send + Sync + Unpin + 'static>(
    hooks: &mut Hooks,
    value: T,
    delay: Duration,
) -> T {
    let debounced = hooks.use_state(|| value.clone());
    
    // Use future to handle debounce
    hooks.use_future({
        let mut debounced = debounced.clone();
        let value = value.clone();
        
        async move {
            smol::Timer::after(delay).await;
            debounced.set(value);
        }
    });
    
    debounced.read().clone()
}

/// Hook that debounces a value, delaying updates until a specified duration has passed
pub fn use_debounced_value<'a, T: Clone + PartialEq + Send + Sync + Unpin + 'static>(
    hooks: &'a mut Hooks,
    value: T,
    delay_ms: u64,
) -> State<T> {
    let mut debounced_value = hooks.use_state(|| value.clone());
    let mut last_update = hooks.use_state(|| None::<Instant>);
    let mut pending_value = hooks.use_state(|| None::<T>);
    
    // Check if value has changed
    let current_value = debounced_value.read().clone();
    if value != current_value {
        // Value changed, set pending update
        pending_value.set(Some(value.clone()));
        last_update.set(Some(Instant::now()));
    }
    
    // Check if we should apply pending update
    hooks.use_future({
        let mut debounced_value = debounced_value.clone();
        let mut pending_value = pending_value.clone();
        let last_update = last_update.clone();
        let delay = Duration::from_millis(delay_ms);
        
        async move {
            let pending_info = match (pending_value.read().clone(), *last_update.read()) {
                (Some(pending), Some(last)) => Some((pending, last)),
                _ => None,
            };
            
            if let Some((_pending, last)) = pending_info {
                let elapsed = Instant::now().duration_since(last);
                if elapsed < delay {
                    // Wait for remaining time
                    smol::Timer::after(delay - elapsed).await;
                }
                
                // Apply the pending value
                let value_to_set = pending_value.read().clone();
                if let Some(value) = value_to_set {
                    debounced_value.set(value);
                    pending_value.set(None);
                }
            }
        }
    });
    
    debounced_value
}

/// Hook that creates a debounced callback function
pub fn use_debounced_callback<F, T>(
    hooks: &mut Hooks,
    callback: F,
    delay_ms: u64,
) -> impl FnMut(T) + 'static
where
    F: Fn(T) + Clone + Send + Sync + Unpin + 'static,
    T: Clone + Send + Sync + Unpin + 'static,
{
    let pending_call = hooks.use_state(|| None::<(T, Instant)>);
    let callback_ref = hooks.use_state(|| Some(callback.clone()));
    
    // Note: We can't update callback_ref after creation
    // because hooks.use_state returns immutable StateRef
    
    // Process pending calls
    hooks.use_future({
        let mut pending_call = pending_call.clone();
        let callback_ref = callback_ref.clone();
        let delay = Duration::from_millis(delay_ms);
        
        async move {
            let pending = pending_call.read().clone();
            if let Some((_value, time)) = pending {
                let elapsed = Instant::now().duration_since(time);
                if elapsed < delay {
                    smol::Timer::after(delay - elapsed).await;
                }
                
                // Execute callback if still pending
                let current_pending = pending_call.read().clone();
                if let Some((pending_value, _)) = current_pending {
                    if let Some(ref cb) = *callback_ref.read() {
                        cb(pending_value);
                    }
                    pending_call.set(None);
                }
            }
        }
    });
    
    // Return a function that queues the callback
    let mut pending_call = pending_call.clone();
    move |value: T| {
        pending_call.set(Some((value, Instant::now())));
    }
}

/// Hook for debounced search functionality
pub fn use_debounced_search<'a>(
    hooks: &'a mut Hooks,
    initial_query: String,
    delay_ms: u64,
) -> (State<String>, State<String>, State<bool>) {
    let input_value = hooks.use_state(|| initial_query);
    let search_query = use_debounced_value(hooks, input_value.read().clone(), delay_ms);
    let mut is_typing = hooks.use_state(|| false);
    
    // Update typing state
    let input_changed = *input_value.read() != *search_query.read();
    if input_changed != *is_typing.read() {
        is_typing.set(input_changed);
    }
    
    (input_value, search_query, is_typing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;
    use smol_macros::test;
    use macro_rules_attribute::apply;
    
    #[component]
    fn TestDebounceComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let counter = hooks.use_state(|| 0);
        let debounced = use_debounce(&mut hooks, *counter.read(), Duration::from_millis(100));
        
        // Increment counter on mount
        hooks.use_future({
            let mut counter = counter.clone();
            async move {
                smol::Timer::after(Duration::from_millis(10)).await;
                counter.set(*counter.read() + 1);
                smol::Timer::after(Duration::from_millis(10)).await;
                counter.set(*counter.read() + 1);
            }
        });
        
        element! {
            Box {
                Text(content: format!("Counter: {}, Debounced: {}", counter.read(), debounced))
            }
        }
    }
    
    #[apply(test!)]
    async fn test_debounced_value() {
        let actual = element!(TestDebounceComponent)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(5)
            .collect::<Vec<_>>()
            .await;
        
        // Initially both should be 0
        assert!(actual[0].contains("Counter: 0, Debounced: 0"));
    }
}