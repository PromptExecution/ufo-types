//! `ViewDefinition` / `ViewUsage` as data — the SysML v2 construct behind
//! *project views*: an [`Expose`] query (which elements), a [`ViewFilter`] list
//! (which kinds survive) and a rendering choice (how they are drawn).
//!
//! Vocabulary is OMG verbatim (`VOCABULARY.md`, D6): the query is `Expose`, the
//! output is a `RenderingUsage`; there is no "projection". [`SysmlViewKind`]
//! stays the *kind* axis (which standard view a definition specializes).
//!
//! # Semantics (SysML v2 / KerML `Import`)
//!
//! | SysML text | [`Expose`] | selects |
//! |---|---|---|
//! | `expose A::B;` | `membership(B)` | `B` |
//! | `expose A::*;` | `members_of(A)` | `A`'s direct members (not `A`) |
//! | `expose A::**;` | `all_under(A)` | every transitive member of `A` (not `A`) |
//! | `expose A::B::**;` | `membership(B).recursive()` | `B` and every transitive member of `B` |
//!
//! `filter @SysML::PartUsage or @SysML::PortUsage;` is one [`ViewFilter`]
//! (`any_of`), several `filter` members are conjunctive. Filters apply to the
//! *exposed set*; they do not stop traversal, so `A::**` with a `PartUsage`
//! filter still reaches parts nested under non-part members.
//!
//! Ownership is read from [`Relation::FeatureMembership`] (`owner` → `member`),
//! the edge kr0ki's lift already emits. This module is **pure data + a pure
//! selector**: it neither parses SysML text nor renders; the selected
//! [`Relation`]s feed an existing renderer (`kr0ki-core::sysml_render`).
//!
//! The four [`StandardRenderingUsage`] values are the ones the normative
//! `Views` package of `sysml.library` defines (`asTextualNotation`,
//! `asTreeDiagram`, `asInterconnectionDiagram`, `asElementTable`).

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::sysml_model::{ElementId, ElementKind, Relation};
use crate::view::SysmlViewKind;

/// A malformed view.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ViewError {
    /// A `filter` with no kinds can never match anything.
    #[error("view `{0}` has a filter with an empty `any_of`")]
    EmptyFilter(String),
    /// A `ViewUsage` was resolved against a definition it does not name.
    #[error("view usage `{usage}` names definition `{named}`, not `{given}`")]
    DefinitionMismatch {
        usage: String,
        named: String,
        given: String,
    },
}

// ── Expose ───────────────────────────────────────────────────────────────────

/// `MembershipExpose` (`expose A::B`) vs `NamespaceExpose` (`expose A::*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExposeForm {
    Membership,
    Namespace,
}

/// One `expose` member of a view.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Expose {
    pub form: ExposeForm,
    /// The exposed element (membership) or namespace (namespace form).
    pub target: ElementId,
    /// `::**` — also expose members of members, transitively.
    #[serde(default)]
    pub recursive: bool,
}

impl Expose {
    /// `expose <target>;`
    pub fn membership(target: ElementId) -> Self {
        Self {
            form: ExposeForm::Membership,
            target,
            recursive: false,
        }
    }

    /// `expose <namespace>::*;`
    pub fn members_of(namespace: ElementId) -> Self {
        Self {
            form: ExposeForm::Namespace,
            target: namespace,
            recursive: false,
        }
    }

    /// `expose <namespace>::**;`
    pub fn all_under(namespace: ElementId) -> Self {
        Self {
            form: ExposeForm::Namespace,
            target: namespace,
            recursive: true,
        }
    }

    /// Make this expose recursive (`expose <target>::**` on a membership).
    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }
}

impl std::fmt::Display for Expose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = self.target.as_str();
        match (self.form, self.recursive) {
            (ExposeForm::Membership, false) => write!(f, "expose {t};"),
            (ExposeForm::Membership, true) | (ExposeForm::Namespace, true) => {
                write!(f, "expose {t}::**;")
            }
            (ExposeForm::Namespace, false) => write!(f, "expose {t}::*;"),
        }
    }
}

// ── Filter ───────────────────────────────────────────────────────────────────

/// One `filter` member: an element survives if its kind is one of `any_of`
/// (`filter @SysML::PartUsage or @SysML::PortUsage;`). Exact kind match; it does
/// not follow specialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ViewFilter {
    pub any_of: Vec<ElementKind>,
}

impl ViewFilter {
    pub fn of(kinds: impl IntoIterator<Item = ElementKind>) -> Self {
        Self {
            any_of: kinds.into_iter().collect(),
        }
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// The standard `RenderingUsage`s of the `Views` library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StandardRenderingUsage {
    AsTextualNotation,
    AsTreeDiagram,
    AsInterconnectionDiagram,
    AsElementTable,
}

/// The `RenderingDefinition` family a standard rendering belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderingFamily {
    Textual,
    Graphical,
    Tabular,
}

impl StandardRenderingUsage {
    /// The identifier in `sysml.library` (`"asTreeDiagram"`).
    pub fn library_name(self) -> &'static str {
        match self {
            Self::AsTextualNotation => "asTextualNotation",
            Self::AsTreeDiagram => "asTreeDiagram",
            Self::AsInterconnectionDiagram => "asInterconnectionDiagram",
            Self::AsElementTable => "asElementTable",
        }
    }

    /// `TextualRendering` / `GraphicalRendering` / `TabularRendering`.
    pub fn family(self) -> RenderingFamily {
        match self {
            Self::AsTextualNotation => RenderingFamily::Textual,
            Self::AsTreeDiagram | Self::AsInterconnectionDiagram => RenderingFamily::Graphical,
            Self::AsElementTable => RenderingFamily::Tabular,
        }
    }

    /// Parse a `sysml.library` identifier (`"asTreeDiagram"`).
    pub fn from_library_name(s: &str) -> Option<Self> {
        [
            Self::AsTextualNotation,
            Self::AsTreeDiagram,
            Self::AsInterconnectionDiagram,
            Self::AsElementTable,
        ]
        .into_iter()
        .find(|r| r.library_name() == s)
    }
}

/// A view's rendering: a standard `RenderingUsage`, or a project-defined one
/// referenced by element id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum RenderingChoice {
    Standard(StandardRenderingUsage),
    Custom(ElementId),
}

// ── ViewDefinition / ViewUsage ───────────────────────────────────────────────

/// `view def` — what to expose, what to keep, how to render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ViewDefinition {
    pub id: ElementId,
    pub name: String,
    /// The standard view kind this definition specializes, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SysmlViewKind>,
    #[serde(default)]
    pub expose: Vec<Expose>,
    #[serde(default)]
    pub filter: Vec<ViewFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<RenderingChoice>,
    /// `ViewpointDefinition` / `ViewpointUsage` elements this view satisfies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub satisfies: Vec<ElementId>,
}

impl ViewDefinition {
    /// Reject malformed content (currently: filters that can never match).
    pub fn validate(&self) -> Result<(), ViewError> {
        if self.filter.iter().any(|f| f.any_of.is_empty()) {
            return Err(ViewError::EmptyFilter(self.name.clone()));
        }
        Ok(())
    }
}

/// `view` — a usage of a [`ViewDefinition`]. It inherits the definition's
/// `expose` and `filter` members and may add its own; its rendering, when
/// present, replaces the definition's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ViewUsage {
    pub id: ElementId,
    pub name: String,
    /// The `ViewDefinition` this usage is typed by.
    pub definition: ElementId,
    #[serde(default)]
    pub expose: Vec<Expose>,
    #[serde(default)]
    pub filter: Vec<ViewFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<RenderingChoice>,
}

impl ViewUsage {
    /// The definition with this usage's additions applied, ready to [`ModelIndex::select`].
    pub fn effective(&self, definition: &ViewDefinition) -> Result<ViewDefinition, ViewError> {
        if definition.id != self.definition {
            return Err(ViewError::DefinitionMismatch {
                usage: self.name.clone(),
                named: self.definition.as_str().to_string(),
                given: definition.id.as_str().to_string(),
            });
        }
        let mut out = definition.clone();
        out.id = self.id.clone();
        out.name = self.name.clone();
        out.expose.extend(self.expose.iter().cloned());
        out.filter.extend(self.filter.iter().cloned());
        if self.rendering.is_some() {
            out.rendering = self.rendering.clone();
        }
        out.validate()?;
        Ok(out)
    }
}

// ── Selection ────────────────────────────────────────────────────────────────

/// What a view selected from a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewSelection {
    /// Selected elements, in the model's own order.
    pub elements: Vec<ElementId>,
    /// Relations whose endpoints are all selected, in the model's own order.
    /// Feed these to a renderer.
    pub relations: Vec<Relation>,
    /// `expose` targets that are not in the model. A view over a stale or partial
    /// snapshot should surface these, not drop them silently.
    pub unresolved: Vec<ElementId>,
}

/// A queryable snapshot: elements with kinds, plus relations.
#[derive(Debug, Clone, Default)]
pub struct ModelIndex {
    order: Vec<ElementId>,
    kinds: HashMap<ElementId, ElementKind>,
    members: HashMap<ElementId, Vec<ElementId>>,
    relations: Vec<Relation>,
}

impl ModelIndex {
    /// Index `elements` (order is preserved, first duplicate wins) and `relations`.
    pub fn new(
        elements: impl IntoIterator<Item = (ElementId, ElementKind)>,
        relations: Vec<Relation>,
    ) -> Self {
        let mut idx = ModelIndex {
            relations,
            ..Default::default()
        };
        for (id, kind) in elements {
            if idx.kinds.insert(id.clone(), kind).is_none() {
                idx.order.push(id);
            }
        }
        for r in &idx.relations {
            if let Relation::FeatureMembership { owner, member } = r {
                let children = idx.members.entry(owner.clone()).or_default();
                if !children.contains(member) {
                    children.push(member.clone());
                }
            }
        }
        idx
    }

    fn descendants(&self, root: &ElementId, out: &mut HashSet<ElementId>) {
        let mut stack = vec![root];
        let mut seen: HashSet<&ElementId> = HashSet::new();
        seen.insert(root);
        while let Some(cur) = stack.pop() {
            for child in self.members.get(cur).into_iter().flatten() {
                if seen.insert(child) {
                    out.insert(child.clone());
                    stack.push(child);
                }
            }
        }
    }

    /// Evaluate a view. Pure and deterministic; cycles in ownership terminate.
    pub fn select(&self, view: &ViewDefinition) -> ViewSelection {
        let mut chosen: HashSet<ElementId> = HashSet::new();
        let mut unresolved = Vec::new();

        for e in &view.expose {
            if !self.kinds.contains_key(&e.target) {
                if !unresolved.contains(&e.target) {
                    unresolved.push(e.target.clone());
                }
                continue;
            }
            match (e.form, e.recursive) {
                (ExposeForm::Membership, false) => {
                    chosen.insert(e.target.clone());
                }
                (ExposeForm::Membership, true) => {
                    chosen.insert(e.target.clone());
                    self.descendants(&e.target, &mut chosen);
                }
                (ExposeForm::Namespace, false) => {
                    chosen.extend(self.members.get(&e.target).into_iter().flatten().cloned());
                }
                (ExposeForm::Namespace, true) => self.descendants(&e.target, &mut chosen),
            }
        }

        let elements: Vec<ElementId> = self
            .order
            .iter()
            .filter(|id| chosen.contains(*id))
            .filter(|id| {
                let kind = self.kinds[*id];
                view.filter.iter().all(|f| f.any_of.contains(&kind))
            })
            .cloned()
            .collect();

        let keep: HashSet<&ElementId> = elements.iter().collect();
        let relations = self
            .relations
            .iter()
            .filter(|r| {
                let ends = r.endpoints();
                !ends.is_empty() && ends.iter().all(|id| keep.contains(*id))
            })
            .cloned()
            .collect();

        ViewSelection {
            elements,
            relations,
            unresolved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ElementId {
        ElementId::new(s)
    }

    fn own(owner: &str, member: &str) -> Relation {
        Relation::FeatureMembership {
            owner: id(owner),
            member: id(member),
        }
    }

    /// Sys (package)
    /// ├── Car (part)
    /// │   ├── Engine (part)
    /// │   │   └── Pump (part)
    /// │   └── Hitch (port)
    /// └── Note (attribute)
    fn model() -> ModelIndex {
        ModelIndex::new(
            [
                (id("Sys"), ElementKind::Package),
                (id("Car"), ElementKind::PartUsage),
                (id("Engine"), ElementKind::PartUsage),
                (id("Pump"), ElementKind::PartUsage),
                (id("Hitch"), ElementKind::PortUsage),
                (id("Note"), ElementKind::AttributeUsage),
            ],
            vec![
                own("Sys", "Car"),
                own("Sys", "Note"),
                own("Car", "Engine"),
                own("Car", "Hitch"),
                own("Engine", "Pump"),
            ],
        )
    }

    fn view(expose: Vec<Expose>, filter: Vec<ViewFilter>) -> ViewDefinition {
        ViewDefinition {
            id: id("v"),
            name: "v".into(),
            kind: None,
            expose,
            filter,
            rendering: None,
            satisfies: vec![],
        }
    }

    fn ids(s: &ViewSelection) -> Vec<&str> {
        s.elements.iter().map(|e| e.as_str()).collect()
    }

    #[test]
    fn membership_selects_exactly_the_target() {
        let s = model().select(&view(vec![Expose::membership(id("Car"))], vec![]));
        assert_eq!(ids(&s), ["Car"]);
    }

    #[test]
    fn namespace_star_selects_direct_members_not_the_namespace() {
        let s = model().select(&view(vec![Expose::members_of(id("Car"))], vec![]));
        assert_eq!(ids(&s), ["Engine", "Hitch"]);
    }

    #[test]
    fn namespace_double_star_selects_all_descendants_not_the_namespace() {
        let s = model().select(&view(vec![Expose::all_under(id("Car"))], vec![]));
        assert_eq!(ids(&s), ["Engine", "Pump", "Hitch"]);
    }

    #[test]
    fn recursive_membership_includes_the_target() {
        let s = model().select(&view(
            vec![Expose::membership(id("Engine")).recursive()],
            vec![],
        ));
        assert_eq!(ids(&s), ["Engine", "Pump"]);
    }

    #[test]
    fn filters_apply_to_the_exposed_set_without_stopping_traversal() {
        let parts = ViewFilter::of([ElementKind::PartUsage]);
        let s = model().select(&view(vec![Expose::all_under(id("Sys"))], vec![parts]));
        assert_eq!(ids(&s), ["Car", "Engine", "Pump"]);
        // Pump is reached through Engine; a port in the middle would not have blocked it.
    }

    #[test]
    fn any_of_is_a_disjunction_and_separate_filters_are_a_conjunction() {
        let either = ViewFilter::of([ElementKind::PartUsage, ElementKind::PortUsage]);
        let s = model().select(&view(
            vec![Expose::all_under(id("Sys"))],
            vec![either.clone()],
        ));
        assert_eq!(ids(&s), ["Car", "Engine", "Pump", "Hitch"]);
        let only_ports = ViewFilter::of([ElementKind::PortUsage]);
        let s = model().select(&view(
            vec![Expose::all_under(id("Sys"))],
            vec![either, only_ports],
        ));
        assert_eq!(ids(&s), ["Hitch"]);
    }

    #[test]
    fn selected_relations_are_the_induced_subgraph() {
        let s = model().select(&view(vec![Expose::all_under(id("Car"))], vec![]));
        // Car itself is not exposed, so the edges owned by Car are not in the subgraph…
        assert_eq!(s.relations, vec![own("Engine", "Pump")]);
        // …until the owner is exposed too.
        let s = model().select(&view(
            vec![Expose::membership(id("Car")).recursive()],
            vec![],
        ));
        assert_eq!(s.relations.len(), 3);
    }

    #[test]
    fn unresolved_targets_are_reported_not_dropped() {
        let s = model().select(&view(
            vec![
                Expose::membership(id("Ghost")),
                Expose::membership(id("Ghost")),
                Expose::membership(id("Car")),
            ],
            vec![],
        ));
        assert_eq!(ids(&s), ["Car"]);
        assert_eq!(s.unresolved, vec![id("Ghost")]);
    }

    #[test]
    fn ownership_cycles_terminate() {
        let m = ModelIndex::new(
            [
                (id("A"), ElementKind::PartUsage),
                (id("B"), ElementKind::PartUsage),
            ],
            vec![own("A", "B"), own("B", "A")],
        );
        let s = m.select(&view(vec![Expose::all_under(id("A"))], vec![]));
        // The namespace itself is never selected by `::**`, even when a cycle makes it its own descendant.
        assert_eq!(ids(&s), ["B"]);
    }

    #[test]
    fn usage_inherits_and_extends_the_definition() {
        let def = ViewDefinition {
            rendering: Some(RenderingChoice::Standard(
                StandardRenderingUsage::AsTreeDiagram,
            )),
            ..view(vec![Expose::members_of(id("Sys"))], vec![])
        };
        let usage = ViewUsage {
            id: id("vu"),
            name: "vu".into(),
            definition: id("v"),
            expose: vec![Expose::membership(id("Pump"))],
            filter: vec![ViewFilter::of([ElementKind::PartUsage])],
            rendering: Some(RenderingChoice::Standard(
                StandardRenderingUsage::AsElementTable,
            )),
        };
        let eff = usage.effective(&def).unwrap();
        assert_eq!(eff.id, id("vu"));
        assert_eq!(eff.expose.len(), 2);
        assert_eq!(
            eff.rendering,
            Some(RenderingChoice::Standard(
                StandardRenderingUsage::AsElementTable
            ))
        );
        let s = model().select(&eff);
        assert_eq!(ids(&s), ["Car", "Pump"]); // Note is filtered out, Pump is added
    }

    #[test]
    fn usage_against_the_wrong_definition_is_rejected() {
        let mut def = view(vec![], vec![]);
        def.id = id("other");
        let usage = ViewUsage {
            id: id("vu"),
            name: "vu".into(),
            definition: id("v"),
            expose: vec![],
            filter: vec![],
            rendering: None,
        };
        assert!(matches!(
            usage.effective(&def),
            Err(ViewError::DefinitionMismatch { .. })
        ));
    }

    #[test]
    fn empty_filter_is_malformed() {
        let v = view(vec![], vec![ViewFilter { any_of: vec![] }]);
        assert!(matches!(v.validate(), Err(ViewError::EmptyFilter(_))));
    }

    #[test]
    fn standard_renderings_match_the_library() {
        use StandardRenderingUsage::*;
        for r in [
            AsTextualNotation,
            AsTreeDiagram,
            AsInterconnectionDiagram,
            AsElementTable,
        ] {
            assert_eq!(
                StandardRenderingUsage::from_library_name(r.library_name()),
                Some(r)
            );
        }
        assert_eq!(AsTreeDiagram.family(), RenderingFamily::Graphical);
        assert_eq!(AsElementTable.family(), RenderingFamily::Tabular);
        assert_eq!(AsTextualNotation.family(), RenderingFamily::Textual);
        assert_eq!(
            StandardRenderingUsage::from_library_name("asPieChart"),
            None
        );
    }

    #[test]
    fn expose_display_uses_sysml_shorthand() {
        assert_eq!(Expose::membership(id("A::B")).to_string(), "expose A::B;");
        assert_eq!(Expose::members_of(id("A")).to_string(), "expose A::*;");
        assert_eq!(Expose::all_under(id("A")).to_string(), "expose A::**;");
        assert_eq!(
            Expose::membership(id("A::B")).recursive().to_string(),
            "expose A::B::**;"
        );
    }

    #[test]
    fn serde_round_trip() {
        let v = ViewDefinition {
            kind: Some(SysmlViewKind::Interconnection),
            rendering: Some(RenderingChoice::Custom(id("MyRendering"))),
            satisfies: vec![id("SafetyViewpoint")],
            ..view(
                vec![Expose::all_under(id("Sys"))],
                vec![ViewFilter::of([ElementKind::PartUsage])],
            )
        };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<ViewDefinition>(&json).unwrap(), v);
    }
}
