//! Selector matching against DOM elements and ancestor chains.

use crate::css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet};
use crate::html::dom::Element;

/// A CSS rule that matched an element, paired with its winning selector's
/// specificity and original stylesheet rule index for tie-breaking.
#[derive(Debug, Clone, Copy)]
pub struct MatchedRule<'a> {
    pub specificity: Specificity,
    pub source_order: usize,
    pub rule: &'a Rule,
}

/// Test whether a single compound selector matches `element`.
pub fn matches_simple_selector(element: &Element, selector: &SimpleSelector) -> bool {
    if let Some(tag) = &selector.tag_name {
        if element.tag_name != *tag {
            return false;
        }
    }
    if let Some(id) = &selector.id {
        if element.id() != Some(id.as_str()) {
            return false;
        }
    }
    selector.classes.iter().all(|cls| element.has_class(cls))
}

/// Test whether a full (possibly descendant-combined) selector matches `element`
/// given its `ancestors` ordered from outermost root (`ancestors[0]`) to direct
/// parent (`ancestors.last()`).
///
/// Matching runs right-to-left: the subject part must match `element`, and each
/// preceding part greedily matches the nearest qualifying ancestor. For pure
/// descendant combinators, greedy innermost matching is exact because it leaves
/// the maximum prefix of outer ancestors available for remaining parts.
pub fn matches_selector(element: &Element, ancestors: &[&Element], selector: &Selector) -> bool {
    let Some((subject, required_ancestors)) = selector.parts.split_last() else {
        return false;
    };
    if !matches_simple_selector(element, subject) {
        return false;
    }

    let mut remaining_ancestors = ancestors.iter().rev();
    for req in required_ancestors.iter().rev() {
        if !remaining_ancestors.any(|anc| matches_simple_selector(anc, req)) {
            return false;
        }
    }
    true
}

/// Collect all stylesheet rules matching `element`, sorted in ascending cascade
/// order `(specificity, source_order)` so later entries override earlier ones.
pub fn matching_rules<'a>(
    element: &Element,
    ancestors: &[&Element],
    stylesheet: &'a Stylesheet,
) -> Vec<MatchedRule<'a>> {
    let mut matched: Vec<MatchedRule<'a>> = stylesheet
        .rules
        .iter()
        .enumerate()
        .filter_map(|(source_order, rule)| {
            // If multiple comma-separated selectors in a single rule match the
            // same element, CSS uses the most specific matching selector.
            let specificity = rule
                .selectors
                .iter()
                .filter(|sel| matches_selector(element, ancestors, sel))
                .map(Selector::specificity)
                .max()?;
            Some(MatchedRule {
                specificity,
                source_order,
                rule,
            })
        })
        .collect();

    matched.sort_by_key(|m| (m.specificity, m.source_order));
    matched
}
