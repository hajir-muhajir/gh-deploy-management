//! Parsing of a workflow's `workflow_dispatch` trigger out of its YAML source.

use yaml_rust2::{Yaml, YamlLoader};

use super::models::DispatchInput;

/// Reads a workflow's `workflow_dispatch` trigger out of its YAML source.
///
/// `None` means the workflow cannot be triggered by hand; `Some(vec![])` means
/// it can, but takes no inputs. REST exposes neither fact, so the file itself is
/// the only source. Kept free of I/O so it can be tested without a token.
pub fn parse_dispatch_inputs(text: &str) -> Option<Vec<DispatchInput>> {
    let docs = YamlLoader::load_from_str(text).ok()?;
    let on = on_node(docs.first()?)?;

    let dispatch = match on {
        // `on: workflow_dispatch`
        Yaml::String(event) => return (event == "workflow_dispatch").then(Vec::new),
        // `on: [push, workflow_dispatch]`
        Yaml::Array(events) => {
            let found = events.iter().any(|e| e.as_str() == Some("workflow_dispatch"));
            return found.then(Vec::new);
        }
        // `on:` followed by an indented block of events.
        Yaml::Hash(_) => field(on, "workflow_dispatch")?,
        _ => return None,
    };

    // A bare `workflow_dispatch:` parses as null, which is dispatchable but
    // input-less — the shape four of the five fixture workflows use.
    let Some(inputs) = field(dispatch, "inputs").and_then(Yaml::as_hash) else {
        return Some(Vec::new());
    };

    Some(inputs.iter().filter_map(|(key, spec)| parse_input(key.as_str()?, spec)).collect())
}

/// The `on:` key of a workflow document.
fn on_node(doc: &Yaml) -> Option<&Yaml> {
    let hash = doc.as_hash()?;
    hash.get(&Yaml::String("on".into())).or_else(|| {
        // A YAML 1.1 parser resolves the bare word `on` to a boolean, which
        // would make every workflow look non-dispatchable. yaml-rust2 is 1.2 so
        // this branch should stay dead, but silence is the wrong failure here.
        hash.get(&Yaml::Boolean(true))
    })
}

/// Hash lookup that treats a missing key and a `BadValue` alike.
fn field<'a>(node: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    node.as_hash()?.get(&Yaml::String(key.into())).filter(|value| !value.is_badvalue())
}

/// Defaults and options may be written unquoted, so they arrive as numbers or
/// booleans. The dispatch API wants strings regardless.
fn scalar(node: &Yaml) -> Option<String> {
    match node {
        Yaml::String(value) => Some(value.clone()),
        Yaml::Real(value) => Some(value.clone()),
        Yaml::Integer(value) => Some(value.to_string()),
        Yaml::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_input(key: &str, spec: &Yaml) -> Option<DispatchInput> {
    let options: Vec<String> = field(spec, "options")
        .and_then(Yaml::as_vec)
        .map(|list| list.iter().filter_map(scalar).collect())
        .unwrap_or_default();

    let kind = match field(spec, "type").and_then(Yaml::as_str).unwrap_or("string") {
        // A choice with no options cannot be drawn as one; fall through to text.
        "choice" if !options.is_empty() => "choice",
        "boolean" => "bool",
        // string, number, environment, and whatever GitHub adds next.
        _ => "text",
    };

    Some(DispatchInput {
        key: key.to_string(),
        label: field(spec, "description").and_then(Yaml::as_str).unwrap_or(key).to_string(),
        kind,
        default: field(spec, "default").and_then(scalar).unwrap_or_default(),
        options,
        required: field(spec, "required").and_then(Yaml::as_bool).unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_form_of_the_on_key() {
        // All four are legal GitHub syntax and all four occur in the wild.
        assert_eq!(
            parse_dispatch_inputs("name: x\non: workflow_dispatch\n").map(|i| i.len()),
            Some(0),
            "scalar form"
        );
        assert_eq!(
            parse_dispatch_inputs("name: x\non: [push, workflow_dispatch]\n").map(|i| i.len()),
            Some(0),
            "array form"
        );
        assert_eq!(
            parse_dispatch_inputs("name: x\non:\n  workflow_dispatch:\n").map(|i| i.len()),
            Some(0),
            "bare block form — four of the five fixture workflows"
        );
        assert_eq!(
            parse_dispatch_inputs(
                "name: x\non:\n  workflow_dispatch:\n    inputs:\n      tag:\n        required: true\n"
            )
            .map(|i| i.len()),
            Some(1),
            "block form with inputs"
        );

        // Not dispatchable: these must be left out of the Run tab entirely.
        assert!(parse_dispatch_inputs("name: x\non:\n  push:\n    branches: [main]\n").is_none());
        assert!(parse_dispatch_inputs("name: x\non: [push, pull_request]\n").is_none());
        assert!(parse_dispatch_inputs("name: x\njobs: {}\n").is_none());
        assert!(parse_dispatch_inputs("::: not yaml at all\n  - [}\n").is_none());
    }

    #[test]
    fn reads_the_on_key_as_a_key_and_not_as_a_boolean() {
        // On YAML 1.1 the bare word `on` resolves to `true`, which would make
        // every workflow look non-dispatchable and quietly empty the tab.
        let docs = YamlLoader::load_from_str("on: workflow_dispatch\n").unwrap();
        let hash = docs[0].as_hash().unwrap();
        assert!(
            hash.contains_key(&Yaml::String("on".into())),
            "the parser must be YAML 1.2; `on` resolved to {:?}",
            hash.keys().next()
        );
    }

    #[test]
    fn reads_input_type_default_and_required() {
        let inputs = parse_dispatch_inputs(
            r#"
name: Release
on:
  workflow_dispatch:
    inputs:
      bump:
        description: Version bump
        type: choice
        default: patch
        options: [patch, minor, major]
      dry_run:
        description: Plan only
        type: boolean
        default: false
      tag:
        description: Tag version to deploy
        required: true
        type: string
      retries:
        type: number
        default: 3
      colour:
        type: choice
"#,
        )
        .expect("workflow_dispatch is present");

        let by_key = |key: &str| inputs.iter().find(|i| i.key == key).unwrap();

        let bump = by_key("bump");
        assert_eq!(bump.kind, "choice");
        assert_eq!(bump.label, "Version bump");
        assert_eq!(bump.default, "patch");
        assert_eq!(bump.options, ["patch", "minor", "major"]);
        assert!(!bump.required);

        let dry_run = by_key("dry_run");
        assert_eq!(dry_run.kind, "bool");
        // Written unquoted, so YAML hands back a boolean — the dispatch API
        // still wants a string.
        assert_eq!(dry_run.default, "false");

        let tag = by_key("tag");
        assert_eq!(tag.kind, "text");
        assert!(tag.required);
        assert_eq!(tag.default, "", "a required input has nothing to prefill");

        assert_eq!(by_key("retries").default, "3", "numbers become strings too");

        let colour = by_key("colour");
        assert_eq!(colour.kind, "text", "a choice without options cannot be a choice");
        assert_eq!(colour.label, "colour", "label falls back to the key");
    }
}
