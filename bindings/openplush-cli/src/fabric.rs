//! The fabric file: one object in the project schema's `fabric` shape.

use openplush_pipeline::{Fabric, FabricCategory, FabricId, FabricSpec, Mm};
use serde_json::{Map, Value};

/// Every field the schema allows; any other is refused, as the schema's
/// `additionalProperties: false` says.
const FIELDS: [&str; 13] = [
    "id",
    "name",
    "category",
    "stretchGrainPct",
    "stretchCrossPct",
    "stretchBiasPct",
    "nap",
    "pileLenMm",
    "thicknessMm",
    "frays",
    "defaultAllowanceMm",
    "rollWidthMm",
    "fromMaterial",
];

/// Reads the fabric, taking the schema's default for each optional field left out, and checks
/// it with `Fabric::new`. A refusal names the field, or says what `Fabric::new` refused.
pub(crate) fn parse(bytes: &[u8]) -> Result<Fabric, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let o = root.as_object().ok_or("the fabric is not a JSON object")?;
    if let Some(unknown) = o.keys().find(|k| !FIELDS.contains(&k.as_str())) {
        return Err(format!("unknown field \"{unknown}\""));
    }
    string(o, "id")?;
    boolean(o, "fromMaterial", false)?;
    let spec = FabricSpec {
        id: FabricId::new(0),
        name: string(o, "name")?.to_owned(),
        category: category(o)?,
        stretch_grain_pct: at_least_zero(o, "stretchGrainPct", 0.0)?,
        stretch_cross_pct: at_least_zero(o, "stretchCrossPct", 0.0)?,
        stretch_bias_pct: at_least_zero(o, "stretchBiasPct", 0.0)?,
        nap: boolean(o, "nap", false)?,
        pile_len: mm(o, "pileLenMm", 0.0)?,
        thickness: mm(o, "thicknessMm", 0.0)?,
        frays: boolean(o, "frays", true)?,
        default_allowance: mm(o, "defaultAllowanceMm", 6.0)?,
        roll_width: roll_width(o)?,
    };
    Fabric::new(spec).map_err(|e| e.to_string())
}

fn string<'a>(o: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    o.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("\"{key}\" is missing or not a string"))
}

fn boolean(o: &Map<String, Value>, key: &str, default: bool) -> Result<bool, String> {
    o.get(key).map_or(Ok(default), |v| {
        v.as_bool()
            .ok_or_else(|| format!("\"{key}\" is not true or false"))
    })
}

fn at_least_zero(o: &Map<String, Value>, key: &str, default: f64) -> Result<f64, String> {
    let n = o.get(key).map_or(Ok(default), |v| {
        v.as_f64()
            .ok_or_else(|| format!("\"{key}\" is not a number"))
    })?;
    if n >= 0.0 {
        Ok(n)
    } else {
        Err(format!("\"{key}\" is {n}, below 0"))
    }
}

fn mm(o: &Map<String, Value>, key: &str, default: f64) -> Result<Mm, String> {
    Mm::try_new(at_least_zero(o, key, default)?).map_err(|e| format!("\"{key}\": {e}"))
}

fn roll_width(o: &Map<String, Value>) -> Result<Mm, String> {
    let width = mm(o, "rollWidthMm", 1500.0)?;
    if width.get() > 0.0 {
        Ok(width)
    } else {
        Err("\"rollWidthMm\" is 0; a roll must have a width".into())
    }
}

fn category(o: &Map<String, Value>) -> Result<FabricCategory, String> {
    match o.get("category").and_then(Value::as_str) {
        Some("woven") => Ok(FabricCategory::Woven),
        Some("knit") => Ok(FabricCategory::Knit),
        Some("pile") => Ok(FabricCategory::Pile),
        Some("felt") => Ok(FabricCategory::Felt),
        _ => Err("\"category\" is not woven, knit, pile or felt".into()),
    }
}
