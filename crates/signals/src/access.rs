//! Reading and writing value cells with every refusal named.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{Error, NodeInfo};
use crate::flush::after_write;
use crate::node::{NodeId, ValueCell};
use crate::propagate::notify_observers;
use crate::runtime::{Runtime, with_runtime};
use crate::track::track_read;

/// Wraps a value for storage in a node.
pub(crate) fn cell<T: 'static>(value: T) -> ValueCell {
    Rc::new(RefCell::new(value))
}

/// The value cell of `id`, refusing a disposed node or a read that
/// would close a cycle; subscribes the running observer when `tracked`.
pub(crate) fn read_cell(id: NodeId, info: NodeInfo, tracked: bool) -> Result<ValueCell, Error> {
    with_runtime(|runtime| {
        refuse_cycle(runtime, id, info)?;
        let cell = runtime
            .node(id)
            .and_then(|node| node.value.clone())
            .ok_or(Error::Disposed { node: info })?;
        if tracked {
            track_read(runtime, id);
        }
        Ok(cell)
    })
}

pub(crate) fn refuse_cycle(runtime: &Runtime, id: NodeId, info: NodeInfo) -> Result<(), Error> {
    let Some(position) = runtime
        .computing
        .iter()
        .position(|computing| *computing == id)
    else {
        return Ok(());
    };
    let mut path: Vec<NodeInfo> = runtime.computing[position..]
        .iter()
        .filter_map(|id| Some(runtime.node(*id)?.info))
        .collect();
    path.push(info);
    Err(Error::Cycle { path })
}

/// Borrows the cell as `T` for `f`.
pub(crate) fn with_cell<T: 'static, R>(
    cell: &ValueCell,
    info: NodeInfo,
    f: impl FnOnce(&T) -> R,
) -> Result<R, Error> {
    let guard = cell.try_borrow().map_err(|_| Error::Busy { node: info })?;
    Ok(f(downcast(&*guard)))
}

/// Writes through `f` and, when `notify` is set, marks the readers and
/// runs effects (unless a batch defers them).
pub(crate) fn write_cell<T: 'static, R>(
    id: NodeId,
    info: NodeInfo,
    notify: bool,
    f: impl FnOnce(&mut T) -> R,
) -> Result<R, Error> {
    let cell = with_runtime(|runtime| {
        runtime
            .node(id)
            .and_then(|node| node.value.clone())
            .ok_or(Error::Disposed { node: info })
    })?;
    let result = {
        let mut guard = cell
            .try_borrow_mut()
            .map_err(|_| Error::Busy { node: info })?;
        f(downcast_mut(&mut *guard))
    };
    if notify {
        with_runtime(|runtime| notify_observers(runtime, id));
        after_write();
    }
    Ok(result)
}

fn downcast<T: 'static>(value: &dyn Any) -> &T {
    value
        .downcast_ref::<T>()
        .expect("a node's value type is fixed when it is created")
}

fn downcast_mut<T: 'static>(value: &mut dyn Any) -> &mut T {
    value
        .downcast_mut::<T>()
        .expect("a node's value type is fixed when it is created")
}
