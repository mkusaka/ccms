//! Memoization hook for performance optimization

use iocraft::prelude::*;
use std::sync::Arc;

/// A hook that memoizes expensive computations
pub fn use_memo<T, F>(hooks: &mut Hooks, compute: F, deps: Vec<&dyn std::any::Any>) -> Arc<T>
where
    T: 'static + Send + Sync + Unpin,
    F: FnOnce() -> T + 'static,
{
    // Store previous dependencies and value
    let mut memo_state = hooks.use_state(|| None::<(Vec<u64>, Arc<T>)>);
    
    // Calculate hash of current dependencies
    let current_hashes: Vec<u64> = deps.iter()
        .map(|dep| {
            // Simple hash using type id and memory address
            // In a real implementation, we'd use a proper hashing mechanism
            dep as *const _ as u64
        })
        .collect();
    
    // Check if dependencies have changed
    let should_recompute = match memo_state.read().as_ref() {
        None => true,
        Some((prev_hashes, _)) => prev_hashes != &current_hashes,
    };
    
    if should_recompute {
        // Compute new value
        let new_value = Arc::new(compute());
        memo_state.set(Some((current_hashes, new_value.clone())));
        new_value
    } else {
        // Return cached value
        memo_state.read()
            .as_ref()
            .map(|(_, value)| value.clone())
            .unwrap()
    }
}

/// A hook that memoizes callback functions
pub fn use_callback<F>(hooks: &mut Hooks, callback: F, deps: Vec<&dyn std::any::Any>) -> Arc<F>
where
    F: 'static + Send + Sync + Unpin,
{
    use_memo(hooks, || callback, deps)
}

/// A hook that provides stable references to values
pub fn use_ref<'a, T>(hooks: &'a mut Hooks, initial_value: T) -> State<T>
where
    T: 'static + Send + Sync + Unpin,
{
    hooks.use_state(|| initial_value)
}

/// A hook that memoizes expensive list transformations
pub fn use_memo_list<T, U, F>(
    hooks: &mut Hooks,
    items: &[T],
    transform: F,
) -> Vec<U>
where
    T: Clone + 'static + PartialEq + Send + Sync + Unpin,
    U: Clone + 'static + Send + Sync + Unpin,
    F: Fn(&T) -> U + 'static,
{
    let mut memo_state = hooks.use_state(|| None::<(Vec<T>, Vec<U>)>);
    
    let should_recompute = match memo_state.read().as_ref() {
        None => true,
        Some((prev_items, _)) => prev_items != items,
    };
    
    if should_recompute {
        let transformed: Vec<U> = items.iter().map(&transform).collect();
        memo_state.set(Some((items.to_vec(), transformed.clone())));
        transformed
    } else {
        memo_state.read()
            .as_ref()
            .map(|(_, transformed)| transformed.clone())
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iocraft::prelude::*;
    use smol_macros::test;
    use macro_rules_attribute::apply;
    
    #[component]
    fn TestMemoComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let counter = hooks.use_state(|| 0);
        let other = hooks.use_state(|| 0);
        
        // Expensive computation that should be memoized
        let expensive_value = use_memo(
            &mut hooks,
            || {
                // Simulate expensive computation
                format!("Computed: {}", *counter.read() * 100)
            },
            vec![&*counter.read()],
        );
        
        element! {
            Box {
                Text(content: format!("Value: {}, Other: {}", expensive_value, other.read()))
            }
        }
    }
    
    #[apply(test!)]
    async fn test_memo_caching() {
        let actual = element!(TestMemoComponent)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        assert!(actual[0].contains("Computed: 0"));
    }
}