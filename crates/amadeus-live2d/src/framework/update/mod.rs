//! Update scheduler and updaters — port of `Framework/src/motion/icubismupdater.ts`,
//! `cubismupdatescheduler.ts` and the `cubism*updater.ts` files (spec §11).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! The scheduler owns boxed updaters; the components they drive are shared with the owner through
//! `Rc<RefCell<…>>` (like the JS references in `LAppModel`). The TS change-listener protocol is
//! replaced by an "is the list still sorted" check before every `on_late_update`; since the sort is
//! stable this is behaviour-identical (re-sorting a sorted list is the identity).

pub mod updaters;

pub use updaters::*;

use crate::framework::parameter_store::ParameterStore;

/// `CubismUpdateOrder` (execution order constants).
pub mod order {
    pub const EYE_BLINK: i64 = 200;
    pub const EXPRESSION: i64 = 300;
    /// `CubismUpdateOrder_Drag` (look).
    pub const DRAG: i64 = 400;
    pub const BREATH: i64 = 500;
    pub const PHYSICS: i64 = 600;
    pub const LIP_SYNC: i64 = 700;
    pub const POSE: i64 = 800;
    /// `Number.MAX_SAFE_INTEGER`.
    pub const MAX: i64 = 9_007_199_254_740_991;
}

/// `ICubismUpdater`.
pub trait CubismUpdater {
    /// `getExecutionOrder()`.
    fn execution_order(&self) -> i64;
    /// `setExecutionOrder(order)` (the scheduler re-sorts before its next update).
    fn set_execution_order(&mut self, execution_order: i64);
    /// `onLateUpdate(model, deltaTimeSeconds)`.
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64);
}

/// `CubismUpdateScheduler`.
#[derive(Default)]
pub struct CubismUpdateScheduler {
    updatable_list: Vec<Box<dyn CubismUpdater>>,
    needs_sort: bool,
}

impl std::fmt::Debug for CubismUpdateScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let orders: Vec<i64> = self
            .updatable_list
            .iter()
            .map(|u| u.execution_order())
            .collect();
        f.debug_struct("CubismUpdateScheduler")
            .field("orders", &orders)
            .field("needs_sort", &self.needs_sort)
            .finish()
    }
}

impl CubismUpdateScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// `addUpdatableList(updatable)`.
    pub fn add_updatable_list(&mut self, updatable: Box<dyn CubismUpdater>) {
        self.updatable_list.push(updatable);
        self.needs_sort = true;
    }

    /// `removeUpdatableList(updatable)` by position.
    pub fn remove_updatable_list(&mut self, index: usize) -> Option<Box<dyn CubismUpdater>> {
        (index < self.updatable_list.len()).then(|| self.updatable_list.remove(index))
    }

    /// `sortUpdatableList()`: stable sort by execution order.
    pub fn sort_updatable_list(&mut self) {
        self.updatable_list.sort_by_key(|u| u.execution_order());
        self.needs_sort = false;
    }

    /// `onLateUpdate(model, deltaTimeSeconds)`: runs every updater in execution order.
    pub fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        if self.needs_sort
            || !self
                .updatable_list
                .is_sorted_by_key(|u| u.execution_order())
        {
            self.sort_updatable_list();
        }
        for updater in &mut self.updatable_list {
            updater.on_late_update(model, delta_time_seconds);
        }
    }

    /// `getUpdatableCount()`.
    pub fn updatable_count(&self) -> usize {
        self.updatable_list.len()
    }

    /// `getUpdatable(index)`.
    pub fn updatable(&self, index: usize) -> Option<&dyn CubismUpdater> {
        self.updatable_list.get(index).map(|u| u.as_ref())
    }

    /// Mutable access (e.g. to change an execution order).
    pub fn updatable_mut(&mut self, index: usize) -> Option<&mut (dyn CubismUpdater + 'static)> {
        self.needs_sort = true;
        self.updatable_list.get_mut(index).map(|u| u.as_mut())
    }

    /// Execution orders in list order.
    pub fn execution_orders(&self) -> Vec<i64> {
        self.updatable_list
            .iter()
            .map(|u| u.execution_order())
            .collect()
    }

    /// `clearUpdatableList()`.
    pub fn clear_updatable_list(&mut self) {
        self.updatable_list.clear();
        self.needs_sort = false;
    }

    /// `release()`.
    pub fn release(&mut self) {
        self.updatable_list.clear();
    }
}
