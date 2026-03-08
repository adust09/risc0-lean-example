//! Filter ir_program.json to only include declarations reachable from entry point.
//! This dramatically reduces the file size for faster loading.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

use serde::Deserialize;
use serde_json::Value as JsonValue;

fn main() {
    let input_path = std::env::args().nth(1).unwrap_or_else(|| "ir_program.json".to_string());
    let output_path = std::env::args().nth(2).unwrap_or_else(|| "ir_program_filtered.json".to_string());
    let entry = std::env::args().nth(3).unwrap_or_else(|| "risc0_main_eth2".to_string());

    eprintln!("Loading {}...", input_path);
    let json_str = fs::read_to_string(&input_path).expect("Failed to read input");

    let mut de = serde_json::Deserializer::from_str(&json_str);
    de.disable_recursion_limit();
    let all_decls: Vec<JsonValue> =
        Vec::deserialize(&mut de).expect("Failed to parse JSON");

    eprintln!("Total declarations: {}", all_decls.len());

    // Build name -> decl map and extract references
    let mut decl_map: HashMap<String, &JsonValue> = HashMap::new();
    let mut references: HashMap<String, Vec<String>> = HashMap::new();

    for decl in &all_decls {
        if let Some(name) = decl.get("name").and_then(|n| n.as_str()) {
            decl_map.insert(name.to_string(), decl);
            let mut refs = Vec::new();
            collect_references(decl, &mut refs);
            references.insert(name.to_string(), refs);
        }
    }

    // BFS from entry point
    let mut reachable: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(entry.clone());

    while let Some(name) = queue.pop_front() {
        if reachable.contains(&name) {
            continue;
        }
        reachable.insert(name.clone());
        if let Some(refs) = references.get(&name) {
            for r in refs {
                if !reachable.contains(r) && decl_map.contains_key(r) {
                    queue.push_back(r.clone());
                }
            }
        }
    }

    eprintln!("Reachable from '{}': {} declarations", entry, reachable.len());

    // Filter
    let filtered: Vec<&JsonValue> = all_decls
        .iter()
        .filter(|d| {
            d.get("name")
                .and_then(|n| n.as_str())
                .map(|n| reachable.contains(n))
                .unwrap_or(false)
        })
        .collect();

    let output_json = serde_json::to_string(&filtered).expect("Failed to serialize");
    fs::write(&output_path, &output_json).expect("Failed to write output");

    let orig_size = json_str.len();
    let new_size = output_json.len();
    eprintln!(
        "Written {} ({:.1}MB -> {:.1}MB, {:.0}% reduction)",
        output_path,
        orig_size as f64 / 1_000_000.0,
        new_size as f64 / 1_000_000.0,
        (1.0 - new_size as f64 / orig_size as f64) * 100.0
    );
}

fn collect_references(v: &JsonValue, refs: &mut Vec<String>) {
    match v {
        JsonValue::Object(map) => {
            // FAp/PAp fun references
            if let Some(kind) = map.get("kind").and_then(|k| k.as_str()) {
                if (kind == "fap" || kind == "pap") {
                    if let Some(fun) = map.get("fun").and_then(|f| f.as_str()) {
                        refs.push(fun.to_string());
                    }
                }
            }
            for (_, val) in map {
                collect_references(val, refs);
            }
        }
        JsonValue::Array(arr) => {
            for val in arr {
                collect_references(val, refs);
            }
        }
        _ => {}
    }
}
