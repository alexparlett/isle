//! A surface in a window of its own, for working on it outside the compositor (B7).

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt;
use isle_tokens::{Appearance, Source, Theme};
use isle_ui::{GlassShapes, ThemeContext};

use crate::{Actions, SurfaceDef, surface::Shown};

/// Every surface the preview can show.
pub fn surfaces() -> Vec<Rc<dyn SurfaceDef>> {
    let probe: Option<Rc<dyn SurfaceDef>> = {
        #[cfg(feature = "probe")]
        {
            Some(Rc::new(crate::probe::Probe::default()))
        }
        #[cfg(not(feature = "probe"))]
        {
            None
        }
    };
    probe.into_iter().collect()
}

/// A surface's tree with what the shell gives it: shown, the tokens of each source that arrives
/// on `sources` resolved for `appearance`, glass that draws its own tint, and actions written to
/// the log.
pub struct Hosted {
    def: Rc<dyn SurfaceDef>,
    appearance: Appearance,
    first: Source,
    sources: Rc<RefCell<Option<UnboundedReceiver<Source>>>>,
}

impl Hosted {
    pub fn new(
        def: Rc<dyn SurfaceDef>,
        appearance: Appearance,
        first: Source,
        sources: UnboundedReceiver<Source>,
    ) -> Self {
        Self {
            def,
            appearance,
            first,
            sources: Rc::new(RefCell::new(Some(sources))),
        }
    }
}

fn resolve(source: &Source, appearance: &Appearance) -> Rc<Theme> {
    Rc::new(source.theme(appearance).unwrap_or_else(|err| {
        tracing::warn!("the tokens do not resolve, the shipped ones stand: {err}");
        Source::shipped()
            .theme(appearance)
            .expect("the shipped tokens resolve")
    }))
}

impl App for Hosted {
    fn render(&self) -> impl IntoElement {
        use_provide_context(|| Shown(State::create(true)));
        let actions = use_provide_context(|| Actions(Default::default()));
        use_provide_context(|| GlassShapes {
            draw_tint: true,
            ..Default::default()
        });
        let theme = use_provide_context(|| {
            ThemeContext(State::create(resolve(&self.first, &self.appearance)))
        });
        use_hook(|| {
            let mut sources = self.sources.borrow_mut().take();
            let appearance = self.appearance.clone();
            spawn(async move {
                let Some(sources) = sources.as_mut() else {
                    return;
                };
                while let Some(source) = sources.next().await {
                    tracing::info!("tokens reloaded");
                    let mut state = theme.0;
                    state.set(resolve(&source, &appearance));
                }
            })
        });
        for action in actions.0.borrow_mut().drain(..) {
            tracing::info!(?action, "the surface asked");
        }
        let app = self.def.app();
        rect().expanded().child(app)
    }
}
