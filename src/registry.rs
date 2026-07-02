use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use crate::model::NativeIndex;
use crate::{NativeError, NativeResult};

type NativeHandle = i64;
type SharedIndex = Arc<Mutex<NativeIndex>>;
type RegistryMap = HashMap<NativeHandle, SharedIndex>;
type RegistryGuard = MutexGuard<'static, RegistryMap>;

static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);
static REGISTRY: LazyLock<Mutex<RegistryMap>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn insert_index(index: NativeIndex) -> NativeResult<NativeHandle> {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    registry()?.insert(handle, Arc::new(Mutex::new(index)));
    Ok(handle)
}

pub(crate) fn remove_index(handle: NativeHandle) -> NativeResult<()> {
    registry()?
        .remove(&handle)
        .map(|_| ())
        .ok_or(NativeError::InvalidHandle(handle))
}

pub(crate) fn with_index<T>(
    handle: NativeHandle,
    action: impl FnOnce(&mut NativeIndex) -> NativeResult<T>,
) -> NativeResult<T> {
    if handle <= 0 {
        return Err(NativeError::InvalidHandle(handle));
    }

    let index = registry()?
        .get(&handle)
        .cloned()
        .ok_or(NativeError::InvalidHandle(handle))?;
    let mut index = index
        .lock()
        .map_err(|_| NativeError::State("native index lock poisoned".to_string()))?;
    action(&mut index)
}

fn registry() -> NativeResult<RegistryGuard> {
    REGISTRY
        .lock()
        .map_err(|_| NativeError::State("native registry lock poisoned".to_string()))
}
