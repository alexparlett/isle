//! An alert or a job's question as a small window of its own, when no Files window is open to
//! hang its sheet from (`p1/structure/conventions.md`, Alert).

use std::rc::Rc;

use freya::prelude::*;

use crate::{
    Shared,
    sheets::{Alert, ConflictSheet, Then, alert_parts},
};

#[derive(Clone)]
pub enum Alone {
    Alert(Alert),
    /// The job waiting on the question.
    Question(u64),
}

#[derive(Clone)]
pub struct AloneWindow {
    pub shared: Rc<Shared>,
    pub what: Alone,
}

impl App for AloneWindow {
    fn render(&self) -> impl IntoElement {
        let key = isle_app::this_window();
        let shared = self.shared.clone();
        let jobs = shared.jobs;
        let job = match &self.what {
            Alone::Question(job) => Some(*job),
            Alone::Alert(_) => None,
        };
        // Answered here or in a window opened since, the question's window goes.
        use_side_effect({
            let shared = shared.clone();
            move || {
                if let Some(job) = job
                    && !jobs.questions.read().iter().any(|q| q.job == job)
                {
                    shared.app().close(key);
                }
            }
        });
        match &self.what {
            Alone::Alert(alert) => {
                let (body, buttons) = alert_parts(alert.clone(), move |then: Option<Then>| {
                    shared.app().close(key);
                    if then == Some(Then::Quit) {
                        shared.quit_now();
                    }
                });
                isle_ui::alert_window(body, buttons)
            }
            Alone::Question(job) => match jobs.questions.read().iter().find(|q| q.job == *job).cloned() {
                Some(question) => ConflictSheet { question, jobs, alone: true }.into_element(),
                None => rect().into_element(),
            },
        }
    }
}
