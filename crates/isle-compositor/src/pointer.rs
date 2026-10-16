//! Pointer lock and confinement, and relative motion (pointer-constraints,
//! relative-pointer).

use smithay::{
    backend::input::InputTime,
    input::pointer::{PointerHandle, RelativeMotionEvent},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
    wayland::{
        compositor::get_parent,
        pointer_constraints::{PointerConstraint, PointerConstraintsHandler, with_pointer_constraint},
    },
};

use crate::{focus::Focus, state::State};

impl PointerConstraintsHandler for State {
    fn new_constraint(&mut self, _surface: &WlSurface, _pointer: &PointerHandle<Self>) {
        self.refresh_constraint();
    }
}

/// What a constraint lets the pointer do.
enum Held {
    Free,
    Locked,
    Confined,
}

fn root(surface: &WlSurface) -> WlSurface {
    let mut root = surface.clone();
    while let Some(parent) = get_parent(&root) {
        root = parent;
    }
    root
}

impl State {
    /// Where a constraint may hold: the surface under the pointer while its window has the
    /// keyboard, with nothing of Isle's holding the pointer and the screen unlocked.
    fn constraint_target(&self) -> Option<(WlSurface, Point<f64, Logical>)> {
        if self.isle.lock.locked || self.picking() {
            return None;
        }
        let pointer = self.isle.seat.get_pointer()?;
        if pointer.is_grabbed() {
            return None;
        }
        let (focus, origin) = self.isle.surface_under(pointer.current_location())?;
        let surface = focus.surface()?.clone();
        let keyboard = self.isle.seat.get_keyboard()?.current_focus()?;
        let focused = keyboard.surface()?;
        (root(&surface) == root(focused)).then_some((surface, origin))
    }

    /// Activates the constraint that may hold now and lets go of any that may not: after
    /// the pointer moves, focus changes, the view moves, the pick opens or closes, the lock.
    pub fn refresh_constraint(&mut self) {
        let Some(pointer) = self.isle.seat.get_pointer() else {
            return;
        };
        let target = self.constraint_target();
        if let Some(active) = self.isle.constrained.clone()
            && target.as_ref().is_none_or(|(surface, _)| *surface != active)
        {
            with_pointer_constraint(&active, &pointer, |constraint| {
                if let Some(constraint) = constraint {
                    constraint.deactivate();
                }
            });
            tracing::info!("pointer constraint released");
            self.isle.constrained = None;
        }
        let Some((surface, origin)) = target else {
            return;
        };
        let at = pointer.current_location() - origin;
        let activated = with_pointer_constraint(&surface, &pointer, |constraint| {
            let Some(constraint) = constraint else {
                return false;
            };
            if !constraint.is_active() {
                if let Some(region) = constraint.region()
                    && !region.contains(at.to_i32_round())
                {
                    return false;
                }
                constraint.activate();
            }
            true
        });
        if activated && self.isle.constrained.as_ref() != Some(&surface) {
            tracing::info!(window = ?self.isle.wm.for_surface(&root(&surface)).map(|r| r.id), "pointer constraint held");
        }
        if activated {
            self.isle.constrained = Some(surface);
        }
    }

    /// Whether a game or app holds the pointer locked or confined.
    pub fn pointer_constrained(&self) -> bool {
        self.isle.constrained.is_some()
    }

    fn held(&self) -> Held {
        let (Some(surface), Some(pointer)) = (&self.isle.constrained, self.isle.seat.get_pointer())
        else {
            return Held::Free;
        };
        with_pointer_constraint(surface, &pointer, |constraint| match constraint {
            Some(c) if c.is_active() => match &*c {
                PointerConstraint::Locked(_) => Held::Locked,
                PointerConstraint::Confined(_) => Held::Confined,
            },
            _ => Held::Free,
        })
    }

    /// Where a move to `target` from `current` may take the pointer: nowhere while locked,
    /// and only within the surface's region while confined.
    pub fn constrained_motion(
        &self,
        current: Point<f64, Logical>,
        target: Point<f64, Logical>,
    ) -> Point<f64, Logical> {
        match self.held() {
            Held::Free => target,
            Held::Locked => current,
            Held::Confined => {
                let Some(surface) = &self.isle.constrained else {
                    return target;
                };
                let inside = self.isle.surface_under(target).is_some_and(|(focus, origin)| {
                    focus.surface() == Some(surface)
                        && self.isle.seat.get_pointer().is_some_and(|pointer| {
                            with_pointer_constraint(surface, &pointer, |constraint| {
                                constraint.and_then(|c| c.region().cloned()).is_none_or(|region| {
                                    region.contains((target - origin).to_i32_round())
                                })
                            })
                        })
                });
                if inside { target } else { current }
            }
        }
    }

    /// Tells the surface under the pointer how far the device moved, accelerated and not.
    pub fn send_relative_motion(
        &mut self,
        delta: Point<f64, Logical>,
        delta_unaccel: Point<f64, Logical>,
        time: InputTime,
    ) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let under: Option<(Focus, _)> = self.isle.surface_under(pointer.current_location());
        pointer.relative_motion(
            self,
            under,
            &RelativeMotionEvent {
                delta,
                delta_unaccel,
                time,
            },
        );
        pointer.frame(self);
    }
}
