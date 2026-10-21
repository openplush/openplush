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

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(category: FabricCategory) -> Result<FabricSpec, openplush_pipeline::PatternError> {
        let mm = |v| {
            Mm::try_new(v).map_err(|source| openplush_pipeline::PatternError::BadScale { source })
        };
        Ok(FabricSpec {
            id: FabricId::new(0),
            name: "F".into(),
            category,
            stretch_grain_pct: 0.0,
            stretch_cross_pct: 0.0,
            stretch_bias_pct: 0.0,
            nap: false,
            pile_len: mm(0.0)?,
            thickness: mm(0.0)?,
            frays: true,
            default_allowance: mm(6.0)?,
            roll_width: mm(1500.0)?,
        })
    }

    #[test]
    fn omitted_fields_take_the_schema_s_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let read = parse(br#"{"id": "f", "name": "F", "category": "felt"}"#)?;
        assert_eq!(read, Fabric::new(spec(FabricCategory::Felt)?)?);
        Ok(())
    }

    #[test]
    fn each_field_reads_into_its_own_place() -> Result<(), Box<dyn std::error::Error>> {
        let read = parse(
            br#"{"id": "f", "name": "F", "category": "knit", "stretchGrainPct": 10,
                "stretchCrossPct": 20, "stretchBiasPct": 30, "nap": true, "pileLenMm": 0,
                "thicknessMm": 1, "frays": false, "defaultAllowanceMm": 8,
                "rollWidthMm": 900, "fromMaterial": true}"#,
        )?;
        let mut want = spec(FabricCategory::Knit)?;
        want.stretch_grain_pct = 10.0;
        want.stretch_cross_pct = 20.0;
        want.stretch_bias_pct = 30.0;
        want.nap = true;
        want.thickness = Mm::try_new(1.0)?;
        want.frays = false;
        want.default_allowance = Mm::try_new(8.0)?;
        want.roll_width = Mm::try_new(900.0)?;
        assert_eq!(read, Fabric::new(want)?);
        Ok(())
    }

    #[test]
    fn each_category_name_reads_as_its_category() {
        for (name, category) in [
            ("woven", FabricCategory::Woven),
            ("knit", FabricCategory::Knit),
            ("pile", FabricCategory::Pile),
            ("felt", FabricCategory::Felt),
        ] {
            let text = format!(r#"{{"id": "f", "name": "F", "category": "{name}"}}"#);
            let read = parse(text.as_bytes()).map(|f| f.spec().category);
            assert_eq!(read, Ok(category), "{name}");
        }
    }

    #[test]
    fn a_missing_id_or_a_mistyped_flag_is_refused_naming_it() {
        for (text, field) in [
            (r#"{"name": "F", "category": "felt"}"#, "id"),
            (
                r#"{"id": "f", "name": "F", "category": "felt", "fromMaterial": "yes"}"#,
                "fromMaterial",
            ),
            (
                r#"{"id": "f", "name": "F", "category": "felt", "stretchGrainPct": -1}"#,
                "stretchGrainPct",
            ),
            ("[]", "object"),
        ] {
            assert!(
                parse(text.as_bytes()).is_err_and(|e| e.contains(field)),
                "{text}"
            );
        }
    }
}
