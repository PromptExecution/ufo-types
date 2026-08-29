//! Bridges ufo-types' domain-generic state-machine shapes to a reusable,
//! standards-based statechart representation — W3C SCXML via the `scxml`
//! crate — so any state machine can be exported, validated, and diagrammed
//! through one shared data model instead of a bespoke per-domain renderer.
//!
//! `scxml` is a document model/interchange layer, not a runtime executor
//! (see its own crate docs). This module never replaces a domain's real
//! transition-enforcement logic (e.g. `OodaStateMachine::dispatch`'s guard
//! checks) — it only gives that logic's *shape* (states, transitions,
//! guards-as-named-data, parallel/hierarchical/history structure) a
//! standard, round-trippable export format, mirroring the compiler-IR
//! analogy `mbse`/`sysml` already establish for other artifact kinds.

use scxml::model::{State, Statechart, Transition};

use crate::dare::{OodaEvent, OodaPhase};

const ALL_PHASES: [OodaPhase; 5] = [
    OodaPhase::Observe,
    OodaPhase::Orient,
    OodaPhase::Decide,
    OodaPhase::Act,
    OodaPhase::Verify,
];

const ALL_EVENTS: [OodaEvent; 8] = [
    OodaEvent::Discover,
    OodaEvent::AnalyzeComplete,
    OodaEvent::Decide,
    OodaEvent::Execute,
    OodaEvent::VerifyComplete,
    OodaEvent::Reject,
    OodaEvent::Retry,
    OodaEvent::Cancel,
];

/// The `Final` state id used for `Cancel` transitions in
/// [`ooda_phases_to_statechart`].
pub const CANCELLED_STATE_ID: &str = "cancelled";

fn phase_id(phase: OodaPhase) -> &'static str {
    match phase {
        OodaPhase::Observe => "observe",
        OodaPhase::Orient => "orient",
        OodaPhase::Decide => "decide",
        OodaPhase::Act => "act",
        OodaPhase::Verify => "verify",
    }
}

/// Export the OODA proposal lifecycle's state graph (`OodaPhase` +
/// `OodaEvent::target`) as a W3C SCXML statechart document.
///
/// This mirrors `OodaEvent::target`'s match table exactly — the same
/// authoritative transition rules `OodaStateMachine::dispatch` consults —
/// with one deliberate addition: `Cancel` is modeled here as a real
/// transition to an explicit `Final` "cancelled" state. The live Rust
/// dispatcher instead treats `Cancel` as an accepted self-loop recorded in
/// `history` (it has no separate phase for termination); the statechart
/// export uses the standard statechart idiom for terminal states instead,
/// since it's a diagrammable model, not the live executor.
pub fn ooda_phases_to_statechart() -> Statechart {
    let mut states: Vec<State> = ALL_PHASES
        .iter()
        .map(|&phase| {
            let mut state = State::atomic(phase_id(phase));
            for &event in &ALL_EVENTS {
                if event == OodaEvent::Cancel {
                    state
                        .transitions
                        .push(Transition::new(event.to_string(), CANCELLED_STATE_ID));
                    continue;
                }
                if let Some(target) = event.target(phase) {
                    state
                        .transitions
                        .push(Transition::new(event.to_string(), phase_id(target)));
                }
            }
            state
        })
        .collect();
    states.push(State::final_state(CANCELLED_STATE_ID));

    Statechart::new(phase_id(OodaPhase::Observe), states).with_name("ooda_proposal_lifecycle")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scxml::export::xml::to_xml;
    use scxml::model::StateKind;
    use scxml::parse_xml;
    use scxml::validate;

    #[test]
    fn ooda_statechart_has_every_phase_plus_a_cancelled_final_state() {
        let chart = ooda_phases_to_statechart();
        assert_eq!(chart.states.len(), 6);
        assert_eq!(
            chart.find_state(CANCELLED_STATE_ID).unwrap().kind,
            StateKind::Final
        );
        for phase in ALL_PHASES {
            assert!(chart.find_state(phase_id(phase)).is_some());
        }
    }

    #[test]
    fn ooda_statechart_discover_reaches_observe_from_every_phase() {
        let chart = ooda_phases_to_statechart();
        for phase in ALL_PHASES {
            let state = chart.find_state(phase_id(phase)).unwrap();
            let discover = state
                .transitions
                .iter()
                .find(|t| t.event.as_ref().map(|e| e.as_str()) == Some("Discover"))
                .expect("every phase must accept Discover per OodaEvent::target");
            assert_eq!(discover.targets, vec!["observe"]);
        }
    }

    #[test]
    fn ooda_statechart_passes_scxml_structural_validation() {
        let chart = ooda_phases_to_statechart();
        validate(&chart).expect("exported OODA statechart should be structurally valid SCXML");
    }

    #[test]
    fn ooda_statechart_round_trips_through_xml_export_and_parse() {
        let chart = ooda_phases_to_statechart();
        let xml = to_xml(&chart);
        let parsed = parse_xml(&xml).expect("exported XML must parse back as valid SCXML");
        assert_eq!(parsed, chart);
    }

    /// A synthetic fixture proving `scxml` represents statechart features
    /// `statig` (the execution-engine dependency already used by
    /// `b00t-c0re-lib::ooda.rs`) cannot: a guard expressed as named data
    /// (not Rust code) and a parallel/orthogonal region.
    #[test]
    fn fixture_with_guard_and_parallel_region_round_trips() {
        let mut a1 = State::atomic("a1");
        a1.transitions
            .push(Transition::new("go", "a2").with_guard("ready"));
        let a2 = State::final_state("a2");

        let mut b1 = State::atomic("b1");
        b1.transitions.push(Transition::new("go", "b2"));
        let b2 = State::final_state("b2");

        let region_a = State::compound("region_a", "a1", vec![a1, a2]);
        let region_b = State::compound("region_b", "b1", vec![b1, b2]);
        let parallel = State::parallel("both_regions", vec![region_a, region_b]);

        let chart =
            Statechart::new("both_regions", vec![parallel]).with_name("fixture_parallel_guard");

        validate(&chart).expect("fixture statechart should be structurally valid");

        let xml = to_xml(&chart);
        let parsed = parse_xml(&xml).expect("fixture XML must parse back");
        assert_eq!(parsed, chart);

        let region_a_parsed = parsed
            .find_state("region_a")
            .and_then(|s| s.children.iter().find(|c| c.id == "a1"))
            .expect("region_a/a1 must survive the round trip");
        assert_eq!(
            region_a_parsed.transitions[0]
                .guard
                .as_ref()
                .map(|g| g.as_str()),
            Some("ready"),
            "guard-as-data must survive export+parse unevaluated"
        );

        let root = &parsed.states[0];
        assert_eq!(root.kind, StateKind::Parallel);
        assert_eq!(root.children.len(), 2);
    }
}
