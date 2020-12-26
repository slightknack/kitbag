use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::{
    content::{
        Contentable,
        Hierarchy,
        Namespace,
    },
    handle::{Location, Identity},
    diff::{Diffable, SetDiff},
};

// TODO: make into trait or enum
// TODO: add keys to agent
/// An `Agent` is a single cryptographically verified identity,
/// That may be controlled by one or more people,
/// Either directly, or through other `Agent`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    hierarchy: Hierarchy<(), Namespace>,
    pub identity: Identity,
    pub display:  String,
}

// TODO: add more functionality
impl Agent {
    /// Creates a new `Agent` with a given display name.
    pub fn new(display: &str) -> Agent {
        todo!()
        // Agent {
        //     display: display.to_string(),
        //     identity: Identity::new(), // Agent
        //     namespaces: HashSet::new(),
        // }
    }
}

impl Contentable for Agent {
    fn context(&self)  -> Location {
        Location::root()
    }

    fn identity(&self) -> Identity {
        todo!()
    }

}

/// An `AgentDiff` represents the difference between two agents.
/// Should be used in the context of a `Delta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiff {
    display: Option<String>,
    namespaces_diff: SetDiff<Identity>,
}

impl Diffable for Agent {
    type Diff = AgentDiff;

    /// Finds the difference between two `Agent`s, and creates an `AgentDiff`.
    fn make(prev: &Agent, next: &Agent) -> AgentDiff {
        let display = if prev.display != next.display { Some(next.display.clone()) }
            else { None };

        let namespaces_diff = Diffable::make(&prev.namespaces, &next.namespaces);
        AgentDiff { display, namespaces_diff }
    }

    /// Applies this diff to another `Agent` to create a new `Agent`.
    fn apply(prev: &Agent, diff: &AgentDiff) -> Agent {
        let display = if let Some(new) = &diff.display { new.to_string() }
            else { prev.display.to_string() };

        let identity = prev.identity.clone();
        let namespaces = Diffable::apply(&prev.namespaces, &diff.namespaces_diff);
        Agent { display, identity, namespaces }
    }
}
