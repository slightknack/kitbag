use serde::{Serialize, Deserialize};

use crate::{
    agent::AgentDiff,
    page::PageDiff,
    namespace::NamespaceDiff,
    storable::Storable
};

// Just a word of warning:
// I tried a lot to make traits work for this project.
// But after hitting a lot of dead ends,
// I realized that they, aside from technical reasons,
// were *very* hard to get working as intended,
// especiall with serde, typetag, and ownership tossed into the mix.
// If you honestly think you can refactor this
// to use traits in an elegant manner, be my guest.
// For now, I find this to work just as well.

#[derive(Clone, Serialize, Deserialize)]
pub enum Diff {
    Agent(AgentDiff),
    Namespace(NamespaceDiff),
    Page(PageDiff),
}

impl Diff {
    fn make(prev: &Storable, next: &Storable) -> Option<Diff> {
        use Storable::*;
        if prev.identity() != next.identity() { return None; }
        let diff: Diff = match (prev, next) {
            ( Agent(p),     Agent(n)     ) => Diff::Agent(AgentDiff::make(p, n)),
            ( Namespace(p), Namespace(n) ) => Diff::Namespace(NamespaceDiff::make(p, n)),
            ( Page(p),      Page(n)      ) => Diff::Page(PageDiff::make(p, n)),
            _ => return None,
        };
        return Some(diff);
    }

    fn apply(tip: &Storable, diff: &Diff) -> Option<Storable> {
        use Storable::*;
        let applied: Storable = match (tip, diff) {
            ( Agent(s),     Diff::Agent(d)     ) => Agent(d.apply(s)),
            ( Namespace(s), Diff::Namespace(d) ) => Namespace(d.apply(s)),
            ( Page(s),      Diff::Page(d)      ) => Page(d.apply(s)),
            _ => return None,
        };
        return Some(applied);
    }
}
