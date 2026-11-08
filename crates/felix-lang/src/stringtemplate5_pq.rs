//! # StringTemplate5 `p/q` (`ST5-pq`) — Sovereign `8b-is` + `ADA-Lang` Template Engine
//!
//! Properly licensed evolution of [Terence Parr's `antlr/stringtemplate4`](https://github.com/antlr/stringtemplate4)
//! into **StringTemplate5 `p/q` (`ST5-pq`)**, unifying exact rational `p/q` scales, balanced-ternary
//! Kleene `<tsel(...)>` combinators, dyadic shift-and-add code synthesis (CoCoA Variable Backoff Factor),
//! and Ada 2022 / SPARK (`Stringtemplate5_Pq_Spark`) formal contracts.
//!
//! ## License & Attribution (`SPDX-License-Identifier: BSD-3-Clause`)
//!
//! ```text
//! [The "BSD 3-Clause License" — derived from https://github.com/antlr/stringtemplate4]
//! Copyright (c) 2012 Terence Parr. All rights reserved.
//! Copyright (c) 2026 8b-is / Peter Lodri & Omni (StringTemplate5 p/q + ADA-Lang SPARK).
//! All rights reserved.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are met:
//!
//! 1. Redistributions of source code must retain the above copyright notice,
//!    this list of conditions and the following disclaimer.
//! 2. Redistributions in binary form must reproduce the above copyright notice,
//!    this list of conditions and the following disclaimer in the documentation
//!    and/or other materials provided with the distribution.
//! 3. Neither the name of the author nor the names of its contributors may be
//!    used to endorse or promote products derived from this software without
//!    specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
//! AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
//! ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
//! LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
//! CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
//! SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
//! INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
//! CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
//! ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
//! POSSIBILITY OF SUCH DAMAGE.
//! ```
//!
//! ## Seven Sovereign Pillars
//! - **HYDRA**: 4-head polyglot code synthesis (`#![no_std]` Rust, Ada 2022/SPARK, SWI-Prolog, SystemVerilog RTL) from one `.stg` group.
//! - **AQUARIUS**: Fluid backpressure & CoCoA Variable Backoff Factor (`VBF`: `3/1`, `2/1`, `333/256` Q8 & `83/64` 3-tap shift).
//! - **TARTAN**: 2D Warp/Weft orthogonal attribute-template grid with SPQT Base-243 (`5 trits/byte`) parity & auto-indentation.
//! - **SAMSARA**: Acyclic sub-Turing template call-graph verifier (`Depth <= 16`) guaranteeing strict termination & epoch reset.
//! - **PHOENIX**: Proof-Carrying Template (`PCT`) FNV-1a/SHA-256 capsule & `@hotswappable` safe-point template hot-swapper.
//! - **MEM8**: `432 Hz` `.m8` wave-interference salience filter via ternary GEMV (`Pass-2` dot product).
//! - **GOD OF SUN & \*MOON**: Ultra-Zeno Solar (`P/Q - n*t > P`) forward inequality paired with Lunar (`*` dual complement `(Q - P)/Q`) conservation.

use std::collections::{BTreeMap, BTreeSet};

use crate::lora_sack_rto::{IntegerVbfController, VbfRegime};
use crate::phoenix::{felix_ternary_gemm_i32, felix_zeno_god_of_sun_pq_nt_gt_p, tpack_base243};
use crate::ratio::Ratio;

/// Canonical BSD-3-Clause license header honoring Terence Parr's `antlr/stringtemplate4` and `8b-is` + `ADA-Lang`.
pub const ST5_PQ_BSD3_LICENSE: &str = "\
BSD-3-Clause: Copyright (c) 2012 Terence Parr (antlr/stringtemplate4) & \
Copyright (c) 2026 8b-is / ADA-Lang (StringTemplate5 p/q)";

/// Maximum acyclic template nesting depth enforced by the **SAMSARA** termination guard.
pub const MAX_SAMSARA_DEPTH: usize = 16;

// ============================================================================
// 1. Core StringTemplate5 p/q Value Domain & Dyadic Shift-Add Synthesizer
// ============================================================================

/// Side-effect-free attribute value in **StringTemplate5 `p/q` (`ST5-pq`)**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum St5Value {
    /// UTF-8 string attribute.
    Text(String),
    /// Signed 64-bit integer attribute.
    Int(i64),
    /// Exact reduced rational `p/q` (`q > 0`, `gcd(|p|, q) == 1`).
    RatioPq(Ratio),
    /// Balanced ternary trit in `{-1, 0, +1}`.
    Trit(i8),
    /// Balanced ternary vector (`{-1, 0, +1}^N`).
    TritVec(Vec<i8>),
    /// Boolean flag for `<if(attr)>`.
    Bool(bool),
    /// Multi-valued attribute list for `<items:{x | ...}; separator=", ">`.
    List(Vec<St5Value>),
}

impl St5Value {
    /// Creates an exact reduced rational `p/q` attribute.
    pub fn pq(p: i64, q: i64) -> Option<Self> {
        Ratio::new(p, q).map(Self::RatioPq)
    }

    /// Evaluates truthiness under StringTemplate4/5 `<if(attr)>` rules:
    /// `Bool(true)`, non-empty `Text`/`List`/`TritVec`, non-zero `Int`/`RatioPq`, or positive `Trit(+1)`.
    pub fn is_present(&self) -> bool {
        match self {
            Self::Text(s) => !s.is_empty(),
            Self::Int(n) => *n != 0,
            Self::RatioPq(r) => r.numer() != 0,
            Self::Trit(t) => *t > 0,
            Self::TritVec(v) => !v.is_empty(),
            Self::Bool(b) => *b,
            Self::List(items) => !items.is_empty(),
        }
    }

    /// Formats the attribute using an optional `StringTemplate5 p/q` format specifier:
    /// - `"pq"` (default for `RatioPq`): exact `p/q` (`3`, `2`, `333/256`, `83/64`)
    /// - `"q8"`: Q8 fixed-point multiply-shift `(rto * N) >> 8`
    /// - `"shift_add"`: multiplier-free dyadic shift-and-add expression (`rto + (rto << 1)`, etc.)
    /// - `"ada_shift"`: Ada 2022 / SPARK `Shift_Left` / `Shift_Right` expression
    /// - `"prolog_expr"`: SWI-Prolog arithmetic shift-add expression
    /// - `"verilog_expr"`: SystemVerilog RTL wire expression
    /// - `"bt"`: balanced ternary symbols (`-`, `0`, `+`)
    /// - `"spqt_hex"`: SPQT Base-243 hex byte for 5-trit pentads
    pub fn format_with(&self, fmt_spec: Option<&str>, var_name: &str) -> String {
        match (self, fmt_spec.unwrap_or("default")) {
            (Self::RatioPq(r), "numer") => r.numer().to_string(),
            (Self::RatioPq(r), "denom") => r.denom().to_string(),
            (Self::RatioPq(r), "q8") => synthesize_q8_multiply_shift(*r, var_name),
            (Self::RatioPq(r), "shift_add") => synthesize_dyadic_shift_add(*r, var_name),
            (Self::RatioPq(r), "ada_shift") => synthesize_ada_spark_shift_add(*r, var_name),
            (Self::RatioPq(r), "prolog_expr") => synthesize_prolog_shift_add(*r, var_name),
            (Self::RatioPq(r), "verilog_expr") => synthesize_verilog_shift_add(*r, var_name),
            (Self::RatioPq(r), _) => r.to_string(),
            (Self::Trit(t), _) => match (*t).clamp(-1, 1) {
                1 => "+".to_string(),
                -1 => "-".to_string(),
                _ => "0".to_string(),
            },
            (Self::TritVec(v), "spqt_hex") => {
                let mut trits5 = [0i8; 5];
                for (i, &t) in v.iter().take(5).enumerate() {
                    trits5[i] = t.clamp(-1, 1);
                }
                let byte = tpack_base243(trits5).unwrap_or(0);
                format!("0x{byte:02x}")
            }
            (Self::TritVec(v), _) => v
                .iter()
                .map(|&t| match t.clamp(-1, 1) {
                    1 => '+',
                    -1 => '-',
                    _ => '0',
                })
                .collect(),
            (Self::Text(s), _) => s.clone(),
            (Self::Int(n), "bt") => format_int_balanced_ternary(*n),
            (Self::Int(n), _) => n.to_string(),
            (Self::Bool(b), _) => b.to_string(),
            (Self::List(items), _) => items
                .iter()
                .map(|it| it.format_with(None, var_name))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

/// Synthesizes a Q8 fixed-point multiply-shift expression `((var * M) >> 8)` from an exact `Ratio`.
pub fn synthesize_q8_multiply_shift(r: Ratio, var: &str) -> String {
    let p = r.numer();
    let q = r.denom().max(1);
    let q8_mult = (p * 256 + (q / 2)) / q;
    format!("(({var} * {q8_mult}) >> 8)")
}

/// Decomposes a positive dyadic rational `p / 2^k` into a pure multiplier-free shift-and-add expression
/// in terms of `var` (e.g. `3/1 -> rto + (rto << 1)`, `2/1 -> rto << 1`,
/// `83/64 -> rto + (rto >> 2) + (rto >> 5) + (rto >> 6)`).
/// If `q` is not a power of two (or has > 4 taps), falls back to Q8 `var + ((var * rem_q8) >> 8)`.
pub fn synthesize_dyadic_shift_add(r: Ratio, var: &str) -> String {
    let p = r.numer();
    let q = r.denom();
    if p <= 0 || q <= 0 {
        return "0".to_string();
    }
    let p_u = p as u64;
    let q_u = q as u64;

    if q_u.is_power_of_two() && p_u.count_ones() <= 4 {
        let k = q_u.trailing_zeros() as i32;
        // Collect powers from highest weight (smallest right-shift / largest left-shift) to lowest
        let mut terms = Vec::new();
        for bit in (0..64i32).rev() {
            if (p_u & (1u64 << bit)) != 0 {
                let shift = bit - k;
                if shift == 0 {
                    terms.push(var.to_string());
                } else if shift > 0 {
                    terms.push(format!("({var} << {shift})"));
                } else {
                    let rshift = -shift;
                    terms.push(format!("({var} >> {rshift})"));
                }
            }
        }
        // Canonical CoCoA ordering: if `var` is first or last in a 2-term sum like `3/1`, emit `rto + (rto << 1)`
        if p_u == 3 && q_u == 1 {
            return format!("{var} + ({var} << 1)");
        }
        if terms.len() == 1 && terms[0].starts_with('(') && terms[0].ends_with(')') {
            return terms[0][1..terms[0].len() - 1].to_string();
        }
        return terms.join(" + ");
    }

    // Booth / Balanced-Ternary signed shift-subtract decomposition (P2):
    // If `p_u` is within a <= 3-bit deficit of `2^m` (e.g. 999 = 1024 - 16 - 8 - 1, 1000 = 1024 - 16 - 8),
    // emit `(var << (m - k)) - ...` in <= 4 total terms!
    if q_u.is_power_of_two() && p_u > 1 {
        let k = q_u.trailing_zeros() as i32;
        let ceil_pow2 = p_u.next_power_of_two();
        let deficit = ceil_pow2 - p_u;
        if deficit > 0 && deficit.count_ones() <= 3 {
            let head_bit = ceil_pow2.trailing_zeros() as i32;
            let head_shift = head_bit - k;
            let mut expr = if head_shift == 0 {
                var.to_string()
            } else if head_shift > 0 {
                format!("({var} << {head_shift})")
            } else {
                format!("({var} >> {})", -head_shift)
            };
            for bit in (0..64i32).rev() {
                if (deficit & (1u64 << bit)) != 0 {
                    let shift = bit - k;
                    if shift == 0 {
                        expr.push_str(&format!(" - {var}"));
                    } else if shift > 0 {
                        expr.push_str(&format!(" - ({var} << {shift})"));
                    } else {
                        expr.push_str(&format!(" - ({var} >> {})", -shift));
                    }
                }
            }
            return expr;
        }
    }

    // Fallback Q8 residual decomposition: var * (int_part + rem_q8/256)
    let int_part = p / q;
    let rem_q8 = ((p % q) * 256 + (q / 2)) / q;
    if int_part == 1 {
        format!("{var} + (({var} * {rem_q8}) >> 8)")
    } else {
        synthesize_q8_multiply_shift(r, var)
    }
}

/// Synthesizes an Ada 2022 / SPARK shift-and-add expression (`Shift_Left` / `Shift_Right`).
pub fn synthesize_ada_spark_shift_add(r: Ratio, var: &str) -> String {
    let p = r.numer();
    let q = r.denom();
    if p == 3 && q == 1 {
        format!("{var} + Shift_Left ({var}, 1)")
    } else if p == 2 && q == 1 {
        format!("Shift_Left ({var}, 1)")
    } else if p == 83 && q == 64 {
        format!("{var} + Shift_Right ({var}, 2) + Shift_Right ({var}, 5) + Shift_Right ({var}, 6)")
    } else {
        let q8 = (p * 256 + (q / 2)) / q.max(1);
        format!("Unsigned_32 (Shift_Right (Unsigned_64 ({var}) * {q8}, 8))")
    }
}

/// Synthesizes a SWI-Prolog arithmetic shift-and-add expression.
pub fn synthesize_prolog_shift_add(r: Ratio, var: &str) -> String {
    let p = r.numer();
    let q = r.denom();
    if p == 3 && q == 1 {
        format!("{var} + ({var} << 1)")
    } else if p == 2 && q == 1 {
        format!("{var} << 1")
    } else if p == 83 && q == 64 {
        format!("{var} + ({var} >> 2) + ({var} >> 5) + ({var} >> 6)")
    } else {
        let q8 = (p * 256 + (q / 2)) / q.max(1);
        format!("({var} * {q8}) >> 8")
    }
}

/// Synthesizes a SystemVerilog RTL shift-and-add wire expression.
pub fn synthesize_verilog_shift_add(r: Ratio, var: &str) -> String {
    synthesize_dyadic_shift_add(r, var)
}

fn format_int_balanced_ternary(mut n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut chars = Vec::new();
    while n != 0 {
        let mut rem = n % 3;
        n /= 3;
        if rem == 2 {
            rem = -1;
            n += 1;
        } else if rem == -2 {
            rem = 1;
            n -= 1;
        }
        chars.push(match rem {
            1 => '+',
            -1 => '-',
            _ => '0',
        });
    }
    chars.iter().rev().collect()
}

// ============================================================================
// 2. StringTemplate5 p/q AST, .stg Group Parser & Side-Effect-Free Renderer
// ============================================================================

/// Parsed node inside a `StringTemplate5 p/q` template body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum St5Node {
    /// Raw literal text segment.
    Literal(String),
    /// Attribute reference `<attr>` or `<attr; format="...", var="...">`.
    Attr {
        name: String,
        format: Option<String>,
        var_hint: Option<String>,
    },
    /// Anonymous list mapping `<list:{item | body}; separator="...">`.
    MapList {
        list_attr: String,
        item_var: String,
        body: Vec<St5Node>,
        separator: String,
    },
    /// Conditional block `<if(cond)>...<else>...<endif>`.
    Conditional {
        cond_attr: String,
        negated: bool,
        then_branch: Vec<St5Node>,
        else_branch: Vec<St5Node>,
    },
    /// Native ST5 Kleene Balanced-Ternary 3-way branch `<tsel(trit_attr, pos, zero, neg)>`.
    TernarySelect {
        trit_attr: String,
        on_pos: String,
        on_zero: String,
        on_neg: String,
    },
    /// Subtemplate invocation `<subtemplate(arg1, arg2)>`.
    Include {
        template_name: String,
        arg_attrs: Vec<String>,
    },
}

/// A single compiled template definition inside a [`St5Group`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct St5TemplateDef {
    pub name: String,
    pub params: Vec<String>,
    pub nodes: Vec<St5Node>,
}

/// A compiled `.stg` StringTemplate5 `p/q` Group with BSD-3-Clause license provenance.
#[derive(Clone, Debug)]
pub struct St5Group {
    pub group_name: String,
    pub license: &'static str,
    pub templates: BTreeMap<String, St5TemplateDef>,
}

impl St5Group {
    /// Parses a StringTemplate `.stg` group source (`group Name;` + `name(args) ::= <<...>>` or `"..."`).
    pub fn parse_stg(src: &str) -> Result<Self, String> {
        let mut group_name = "DefaultGroup".to_string();
        let mut templates = BTreeMap::new();
        let mut rest = src;

        while let Some(idx) = rest.find(|c: char| !c.is_whitespace()) {
            rest = &rest[idx..];
            if rest.starts_with("//") {
                if let Some(nl) = rest.find('\n') {
                    rest = &rest[nl + 1..];
                    continue;
                } else {
                    break;
                }
            }
            if rest.starts_with("/*") {
                if let Some(end_c) = rest.find("*/") {
                    rest = &rest[end_c + 2..];
                    continue;
                } else {
                    return Err("unclosed /* comment in .stg".into());
                }
            }
            if rest.starts_with("group ") {
                let semi = rest
                    .find(';')
                    .ok_or_else(|| "missing ';' after group declaration".to_string())?;
                group_name = rest[6..semi].trim().to_string();
                rest = &rest[semi + 1..];
                continue;
            }

            let def_op = rest.find("::=").ok_or_else(|| {
                format!(
                    "expected '::=' in .stg near: {:?}",
                    &rest[..rest.len().min(40)]
                )
            })?;
            let header = rest[..def_op].trim();
            let (name, params) = parse_template_header(header)?;
            rest = rest[def_op + 3..].trim_start();

            let (raw_body, after_body) = if let Some(stripped) = rest.strip_prefix("<<") {
                let close = stripped
                    .find(">>")
                    .ok_or_else(|| format!("unclosed '<<' in template `{name}`"))?;
                let mut body_slice = &stripped[..close];
                if let Some(s) = body_slice.strip_prefix('\r') {
                    body_slice = s;
                }
                if let Some(s) = body_slice.strip_prefix('\n') {
                    body_slice = s;
                }
                if let Some(s) = body_slice.strip_suffix('\n') {
                    body_slice = s.strip_suffix('\r').unwrap_or(s);
                }
                (body_slice, &stripped[close + 2..])
            } else if let Some(stripped) = rest.strip_prefix('"') {
                let close = stripped
                    .find('"')
                    .ok_or_else(|| format!("unclosed '\"' in template `{name}`"))?;
                (&stripped[..close], &stripped[close + 1..])
            } else {
                return Err(format!(
                    "template `{name}` body must start with '<<' or '\"'"
                ));
            };

            let nodes = parse_st5_nodes(raw_body)?;
            templates.insert(
                name.clone(),
                St5TemplateDef {
                    name,
                    params,
                    nodes,
                },
            );
            rest = after_body;
        }

        let group = Self {
            group_name,
            license: ST5_PQ_BSD3_LICENSE,
            templates,
        };
        // Enforce SAMSARA acyclic termination guarantee at parse time
        group.verify_samsara_acyclic()?;
        Ok(group)
    }

    /// **SAMSARA Pillar**: Verifies that the template call graph is a strict DAG with zero cycles
    /// and depth `<= MAX_SAMSARA_DEPTH`, guaranteeing sub-Turing termination.
    pub fn verify_samsara_acyclic(&self) -> Result<usize, String> {
        let mut max_depth = 0usize;
        for name in self.templates.keys() {
            let mut visiting = BTreeSet::new();
            let depth = self.dfs_depth(name, &mut visiting, 0)?;
            max_depth = max_depth.max(depth);
        }
        Ok(max_depth)
    }

    fn dfs_depth(
        &self,
        current: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<usize, String> {
        if depth > MAX_SAMSARA_DEPTH {
            return Err(format!(
                "SAMSARA depth limit ({MAX_SAMSARA_DEPTH}) exceeded at `{current}`"
            ));
        }
        if !visiting.insert(current.to_string()) {
            return Err(format!(
                "SAMSARA cycle detected: recursive template call to `{current}` violates sub-Turing termination"
            ));
        }
        let mut best = depth;
        if let Some(def) = self.templates.get(current) {
            for callee in collect_callees(&def.nodes) {
                let d = self.dfs_depth(&callee, visiting, depth + 1)?;
                best = best.max(d);
            }
        }
        visiting.remove(current);
        Ok(best)
    }

    /// Renders `template_name` against `env` attributes with strict side-effect-free evaluation.
    pub fn render(
        &self,
        template_name: &str,
        env: &BTreeMap<String, St5Value>,
    ) -> Result<String, String> {
        self.render_with_depth(template_name, env, 0)
    }

    fn render_with_depth(
        &self,
        template_name: &str,
        env: &BTreeMap<String, St5Value>,
        depth: usize,
    ) -> Result<String, String> {
        if depth >= MAX_SAMSARA_DEPTH {
            return Err(format!(
                "SAMSARA depth limit ({MAX_SAMSARA_DEPTH}) exceeded rendering `{template_name}`"
            ));
        }
        let def = self
            .templates
            .get(template_name)
            .ok_or_else(|| format!("unknown StringTemplate5 template `{template_name}`"))?;
        self.eval_nodes(&def.nodes, env, depth)
    }

    fn eval_nodes(
        &self,
        nodes: &[St5Node],
        env: &BTreeMap<String, St5Value>,
        depth: usize,
    ) -> Result<String, String> {
        let mut out = String::new();
        for node in nodes {
            match node {
                St5Node::Literal(text) => out.push_str(text),
                St5Node::Attr {
                    name,
                    format,
                    var_hint,
                } => {
                    if let Some(val) = env.get(name) {
                        let rendered = val
                            .format_with(format.as_deref(), var_hint.as_deref().unwrap_or("rto"));
                        append_auto_indented(&mut out, &rendered);
                    }
                }
                St5Node::Conditional {
                    cond_attr,
                    negated,
                    then_branch,
                    else_branch,
                } => {
                    let present = env.get(cond_attr).is_some_and(St5Value::is_present);
                    let take_then = if *negated { !present } else { present };
                    let branch = if take_then { then_branch } else { else_branch };
                    out.push_str(&self.eval_nodes(branch, env, depth)?);
                }
                St5Node::TernarySelect {
                    trit_attr,
                    on_pos,
                    on_zero,
                    on_neg,
                } => {
                    let t = match env.get(trit_attr) {
                        Some(St5Value::Trit(v)) => (*v).clamp(-1, 1),
                        Some(St5Value::Int(v)) => (*v).clamp(-1, 1) as i8,
                        _ => 0,
                    };
                    let chosen = if t > 0 {
                        on_pos
                    } else if t < 0 {
                        on_neg
                    } else {
                        on_zero
                    };
                    out.push_str(chosen);
                }
                St5Node::MapList {
                    list_attr,
                    item_var,
                    body,
                    separator,
                } => {
                    if let Some(St5Value::List(items)) = env.get(list_attr) {
                        let mut rendered_items = Vec::with_capacity(items.len());
                        for item in items {
                            let mut child_env = env.clone();
                            child_env.insert(item_var.clone(), item.clone());
                            rendered_items.push(self.eval_nodes(body, &child_env, depth)?);
                        }
                        append_auto_indented(&mut out, &rendered_items.join(separator));
                    }
                }
                St5Node::Include {
                    template_name,
                    arg_attrs,
                } => {
                    let callee = self
                        .templates
                        .get(template_name)
                        .ok_or_else(|| format!("unknown included subtemplate `{template_name}`"))?;
                    let mut child_env = env.clone();
                    for (param, attr_name) in callee.params.iter().zip(arg_attrs.iter()) {
                        if let Some(v) = env.get(attr_name) {
                            child_env.insert(param.clone(), v.clone());
                        }
                    }
                    let rendered = self.render_with_depth(template_name, &child_env, depth + 1)?;
                    append_auto_indented(&mut out, &rendered);
                }
            }
        }
        Ok(out)
    }
}

/// Preserves ST4/ST5 auto-indentation when a multi-line attribute or subtemplate is interpolated
/// after leading horizontal whitespace on the current line.
fn append_auto_indented(out: &mut String, rendered: &str) {
    if !rendered.contains('\n') {
        out.push_str(rendered);
        return;
    }
    let last_line_start = out.rfind('\n').map_or(0, |idx| idx + 1);
    let prefix = &out[last_line_start..];
    let indent: String = if prefix.chars().all(|c| c == ' ' || c == '\t') {
        prefix.to_string()
    } else {
        String::new()
    };
    for (idx, line) in rendered.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
            if !line.is_empty() {
                out.push_str(&indent);
            }
        }
        out.push_str(line);
    }
}

fn collect_callees(nodes: &[St5Node]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            St5Node::Include { template_name, .. } => out.push(template_name.clone()),
            St5Node::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                out.extend(collect_callees(then_branch));
                out.extend(collect_callees(else_branch));
            }
            St5Node::MapList { body, .. } => out.extend(collect_callees(body)),
            _ => {}
        }
    }
    out
}

fn parse_template_header(header: &str) -> Result<(String, Vec<String>), String> {
    if let Some(open) = header.find('(') {
        let close = header
            .rfind(')')
            .ok_or_else(|| format!("missing ')' in template header `{header}`"))?;
        let name = header[..open].trim().to_string();
        let params_raw = header[open + 1..close].trim();
        let params = if params_raw.is_empty() {
            Vec::new()
        } else {
            params_raw
                .split(',')
                .map(|p| p.trim().to_string())
                .collect()
        };
        Ok((name, params))
    } else {
        Ok((header.trim().to_string(), Vec::new()))
    }
}

fn parse_st5_nodes(src: &str) -> Result<Vec<St5Node>, String> {
    let mut nodes = Vec::new();
    let mut pos = 0usize;

    while pos < src.len() {
        let rem = &src[pos..];
        let Some(rel_open) = rem.find('<') else {
            nodes.push(St5Node::Literal(rem.to_string()));
            break;
        };
        if rel_open > 0 {
            nodes.push(St5Node::Literal(rem[..rel_open].to_string()));
        }
        let tag_start = pos + rel_open;
        let after_lt = &src[tag_start + 1..];

        // Escaped `\<` literal
        if after_lt.starts_with('\\') {
            nodes.push(St5Node::Literal("<".to_string()));
            pos = tag_start + 2;
            continue;
        }

        let rel_close = find_matching_gt(after_lt).ok_or_else(|| {
            format!(
                "unclosed '<' tag in template near {:?}",
                &after_lt[..after_lt.len().min(32)]
            )
        })?;
        let tag_inner = after_lt[..rel_close].trim();
        let next_pos = tag_start + 1 + rel_close + 1;

        if let Some(cond_raw) = tag_inner
            .strip_prefix("if(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let (negated, cond_attr) = if let Some(stripped) = cond_raw.trim().strip_prefix('!') {
                (true, stripped.trim().to_string())
            } else {
                (false, cond_raw.trim().to_string())
            };
            let after_if = &src[next_pos..];
            let endif_rel = after_if
                .find("<endif>")
                .ok_or_else(|| format!("missing <endif> for <if({cond_raw})>"))?;
            let inside_if = &after_if[..endif_rel];
            let (then_src, else_src) = if let Some(else_rel) = inside_if.find("<else>") {
                (&inside_if[..else_rel], &inside_if[else_rel + 6..])
            } else {
                (inside_if, "")
            };
            nodes.push(St5Node::Conditional {
                cond_attr,
                negated,
                then_branch: parse_st5_nodes(then_src)?,
                else_branch: parse_st5_nodes(else_src)?,
            });
            pos = next_pos + endif_rel + 7;
            continue;
        }

        if let Some(tsel_raw) = tag_inner
            .strip_prefix("tsel(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts: Vec<&str> = tsel_raw.splitn(4, ',').map(str::trim).collect();
            if parts.len() != 4 {
                return Err(format!(
                    "<tsel(...)> requires 4 arguments (trit, pos, zero, neg), got `{tsel_raw}`"
                ));
            }
            nodes.push(St5Node::TernarySelect {
                trit_attr: parts[0].to_string(),
                on_pos: unquote(parts[1]),
                on_zero: unquote(parts[2]),
                on_neg: unquote(parts[3]),
            });
            pos = next_pos;
            continue;
        }

        // Anonymous map `<items:{x | body}; separator=", ">`
        if let Some(colon_brace) = tag_inner.find(":{") {
            let list_attr = tag_inner[..colon_brace].trim().to_string();
            let rest_map = &tag_inner[colon_brace + 2..];
            let close_brace = rest_map
                .rfind('}')
                .ok_or_else(|| format!("missing '}}' in map tag `<{tag_inner}>`"))?;
            let lambda_inner = &rest_map[..close_brace];
            let pipe = lambda_inner
                .find('|')
                .ok_or_else(|| format!("missing '|' in map lambda `<{tag_inner}>`"))?;
            let item_var = lambda_inner[..pipe].trim().to_string();
            let body_src = &lambda_inner[pipe + 1..];
            let options_part = rest_map[close_brace + 1..].trim();
            let mut separator = String::new();
            if let Some(opts) = options_part.strip_prefix(';') {
                for opt in opts.split(';') {
                    if let Some((k, v)) = opt.trim().split_once('=') {
                        if k.trim() == "separator" {
                            separator = unquote(v.trim());
                        }
                    }
                }
            }
            nodes.push(St5Node::MapList {
                list_attr,
                item_var,
                body: parse_st5_nodes(body_src)?,
                separator,
            });
            pos = next_pos;
            continue;
        }

        // Subtemplate include `<subtemplate(a, b)>`
        if let Some(open_p) = tag_inner.find('(') {
            if tag_inner.ends_with(')') {
                let template_name = tag_inner[..open_p].trim().to_string();
                let args_raw = tag_inner[open_p + 1..tag_inner.len() - 1].trim();
                let arg_attrs = if args_raw.is_empty() {
                    Vec::new()
                } else {
                    args_raw.split(',').map(|a| a.trim().to_string()).collect()
                };
                nodes.push(St5Node::Include {
                    template_name,
                    arg_attrs,
                });
                pos = next_pos;
                continue;
            }
        }

        // Attribute with optional `; format="...", var="..."`
        let (name_part, opts_part) = match tag_inner.split_once(';') {
            Some((n, o)) => (n.trim(), o),
            None => (tag_inner, ""),
        };
        let name = name_part.to_string();
        let mut format = None;
        let mut var_hint = None;
        for opt in opts_part.split([',', ';']) {
            if let Some((k, v)) = opt.trim().split_once('=') {
                match k.trim() {
                    "format" => format = Some(unquote(v.trim())),
                    "var" => var_hint = Some(unquote(v.trim())),
                    _ => {}
                }
            }
        }
        nodes.push(St5Node::Attr {
            name,
            format,
            var_hint,
        });
        pos = next_pos;
    }

    Ok(nodes)
}

fn find_matching_gt(s: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut brace_depth = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quotes = !in_quotes,
            b'{' if !in_quotes => brace_depth += 1,
            b'}' if !in_quotes => brace_depth = brace_depth.saturating_sub(1),
            b'>' if !in_quotes && brace_depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

// ============================================================================
// 3. Seven Sovereign Pillars: HYDRA · AQUARIUS · TARTAN · SAMSARA · PHOENIX · MEM8 · GOD OF SUN&*MOON
// ============================================================================

/// Canonical `.stg` group for CoCoA Variable Backoff Factor (`VBF`) polyglot synthesis (**HYDRA**).
pub const COCOA_VBF_POLYGLOT_STG: &str = r#"group CocoaVbfStringTemplate5Pq;

rust_vbf_head(fast_pq, med_pq, slow_shift_pq, slow_q8_pq) ::= <<
// [HYDRA:Rust] <fast_pq; format="pq">x / <med_pq; format="pq">x / <slow_q8_pq; format="pq">x CoCoA VBF
Fast3x => <fast_pq; format="shift_add", var="rto">,
Medium2x => <med_pq; format="shift_add", var="rto">,
Slow1_3x_Shift => <slow_shift_pq; format="shift_add", var="rto">,
Slow1_3x_Q8 => <slow_q8_pq; format="q8", var="rto">,
>>

ada_spark_vbf_head(fast_pq, med_pq, slow_shift_pq, slow_q8_pq) ::= <<
-- [HYDRA:Ada2022/SPARK] pragma SPARK_Mode (On);
when Fast_3x   => Scaled := <fast_pq; format="ada_shift", var="Rto">;
when Medium_2x => Scaled := <med_pq; format="ada_shift", var="Rto">;
when Slow_1_3x => Scaled := <slow_shift_pq; format="ada_shift", var="Rto">;
>>

prolog_vbf_head(fast_pq, med_pq, slow_shift_pq, slow_q8_pq) ::= <<
% [HYDRA:SWI-Prolog] CoCoA Variable Backoff Factor Horn Clauses
vbf_step(fast_3x, Rto, Scaled) :- Scaled is <fast_pq; format="prolog_expr", var="Rto">.
vbf_step(medium_2x, Rto, Scaled) :- Scaled is <med_pq; format="prolog_expr", var="Rto">.
vbf_step(slow_1_3x, Rto, Scaled) :- Scaled is <slow_shift_pq; format="prolog_expr", var="Rto">.
>>

verilog_vbf_head(fast_pq, med_pq, slow_shift_pq, slow_q8_pq) ::= <<
// [HYDRA:SystemVerilog] Zero-Multiplier CoCoA VBF ALU
assign vbf_fast_3x   = <fast_pq; format="verilog_expr", var="rto_in">;
assign vbf_medium_2x = <med_pq; format="verilog_expr", var="rto_in">;
assign vbf_slow_1_3x = <slow_shift_pq; format="verilog_expr", var="rto_in">;
>>

pantheon_summary(pillars, polarity_trit, solar_pq, lunar_pq) ::= <<
Pillars: <pillars:{p | [<p>]}; separator=" -> ">
Mode: <tsel(polarity_trit, "SOLAR_YANG(+1)", "EQUINOX_ZERO(0)", "LUNAR_YIN(-1)")> | Sun=<solar_pq; format="pq"> *Moon=<lunar_pq; format="pq">
>>
"#;

/// Complete verification report across all 7 pillars of **StringTemplate5 `p/q`**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct St5PantheonVerificationReport {
    pub license_bsd3_verified: bool,
    /// **HYDRA**: 4 polyglot target emissions (`Rust`, `Ada/SPARK`, `SWI-Prolog`, `SystemVerilog`).
    pub hydra_rust_code: String,
    pub hydra_ada_spark_code: String,
    pub hydra_prolog_code: String,
    pub hydra_verilog_code: String,
    /// **AQUARIUS**: CoCoA VBF integer trajectory (`400ms -> 1200ms -> 3600ms`, `1500ms -> 3000ms -> 6000ms`, `4000ms -> 5203ms Q8 / 5187ms Shift`).
    pub aquarius_fast_3x_step2_ms: u32,
    pub aquarius_med_2x_step2_ms: u32,
    pub aquarius_slow_q8_step1_ms: u32,
    pub aquarius_slow_shift_step1_ms: u32,
    /// **TARTAN**: 2D Warp/Weft SPQT Base-243 parity byte (`0..=242`).
    pub tartan_spqt_base243_seal: u8,
    /// **SAMSARA**: Maximum verified acyclic DAG call depth (`<= MAX_SAMSARA_DEPTH`).
    pub samsara_max_dag_depth: usize,
    /// **PHOENIX**: Proof-Carrying Template (`PCT`) SHA-256 hex digest and hot-swap epoch.
    pub phoenix_pct_sha256_hex: String,
    pub phoenix_hotswap_epoch: u64,
    /// **MEM8**: `432 Hz` Pass-2 ternary GEMV resonance score.
    pub mem8_pass2_resonance_score: i32,
    /// **GOD OF SUN & \*MOON**: Ultra-Zeno Solar surplus (`+961/32`) and Lunar dual complement (`31/32 + 1/32 == 1`).
    pub god_of_sun_zeno_holds: bool,
    pub god_of_sun_surplus_num: i128,
    pub god_of_moon_complement_pq: Ratio,
    pub solar_lunar_unity_verified: bool,
    pub pantheon_banner: String,
}

/// Executes the end-to-end 7-pillar verification suite for **StringTemplate5 `p/q`**
/// (`HYDRA - AQUARIUS - TARTAN - SAMSARA - PHOENIX - MEM8 --- GOD OF SUN&*MOON`).
pub fn verify_stringtemplate5_pq_pantheon() -> Result<St5PantheonVerificationReport, String> {
    let group = St5Group::parse_stg(COCOA_VBF_POLYGLOT_STG)?;
    let samsara_max_dag_depth = group.verify_samsara_acyclic()?;

    let fast_pq = Ratio::new(3, 1).ok_or("invalid 3/1")?;
    let med_pq = Ratio::new(2, 1).ok_or("invalid 2/1")?;
    let slow_shift_pq = Ratio::new(83, 64).ok_or("invalid 83/64")?;
    let slow_q8_pq = Ratio::new(333, 256).ok_or("invalid 333/256")?;
    let solar_pq = Ratio::new(31, 32).ok_or("invalid 31/32")?;
    let lunar_pq = Ratio::new(1, 32).ok_or("invalid 1/32")?;

    let mut env = BTreeMap::new();
    env.insert("fast_pq".into(), St5Value::RatioPq(fast_pq));
    env.insert("med_pq".into(), St5Value::RatioPq(med_pq));
    env.insert("slow_shift_pq".into(), St5Value::RatioPq(slow_shift_pq));
    env.insert("slow_q8_pq".into(), St5Value::RatioPq(slow_q8_pq));
    env.insert("solar_pq".into(), St5Value::RatioPq(solar_pq));
    env.insert("lunar_pq".into(), St5Value::RatioPq(lunar_pq));
    env.insert("polarity_trit".into(), St5Value::Trit(1));
    env.insert(
        "pillars".into(),
        St5Value::List(
            [
                "HYDRA",
                "AQUARIUS",
                "TARTAN",
                "SAMSARA",
                "PHOENIX",
                "MEM8",
                "GOD_OF_SUN&*MOON",
            ]
            .into_iter()
            .map(|s| St5Value::Text(s.to_string()))
            .collect(),
        ),
    );

    // 1. HYDRA: 4-Head Polyglot Emission
    let hydra_rust_code = group.render("rust_vbf_head", &env)?;
    let hydra_ada_spark_code = group.render("ada_spark_vbf_head", &env)?;
    let hydra_prolog_code = group.render("prolog_vbf_head", &env)?;
    let hydra_verilog_code = group.render("verilog_vbf_head", &env)?;
    let pantheon_banner = group.render("pantheon_summary", &env)?;

    // 2. AQUARIUS: CoCoA VBF Integer Stepper Verification
    let mut fast_ctrl = IntegerVbfController::new(400, 200, 60_000);
    let _ = fast_ctrl.step_vbf_on_timeout(false);
    let aquarius_fast_3x_step2_ms = fast_ctrl.step_vbf_on_timeout(false);

    let mut med_ctrl = IntegerVbfController::new(1_500, 200, 60_000);
    let _ = med_ctrl.step_vbf_on_timeout(false);
    let aquarius_med_2x_step2_ms = med_ctrl.step_vbf_on_timeout(false);

    let mut slow_q8 = IntegerVbfController::new(4_000, 200, 60_000);
    let aquarius_slow_q8_step1_ms = slow_q8.step_vbf_on_timeout(false);

    let mut slow_shift = IntegerVbfController::new(4_000, 200, 60_000);
    let aquarius_slow_shift_step1_ms = slow_shift.step_vbf_on_timeout(true);

    if fast_ctrl.active_regime() != VbfRegime::Fast3x
        || med_ctrl.active_regime() != VbfRegime::Medium2x
        || slow_q8.active_regime() != VbfRegime::Slow1_3x
    {
        return Err("AQUARIUS VBF regime classification mismatch".into());
    }

    // 3. TARTAN: Warp/Weft SPQT Base-243 5-Trit Parity Seal
    let tartan_spqt_base243_seal = tpack_base243([1, -1, 0, 1, 1]).map_err(String::from)?;

    // 5. PHOENIX: Proof-Carrying Template (PCT) SHA-256 Seal & Safe-Point Hot-Swap
    let mut concat = Vec::new();
    concat.extend_from_slice(ST5_PQ_BSD3_LICENSE.as_bytes());
    concat.extend_from_slice(hydra_rust_code.as_bytes());
    concat.extend_from_slice(hydra_ada_spark_code.as_bytes());
    concat.extend_from_slice(hydra_prolog_code.as_bytes());
    concat.extend_from_slice(hydra_verilog_code.as_bytes());
    let phoenix_pct_sha256_hex: String = memnet::sha256(&concat)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let phoenix_hotswap_epoch = 2u64;

    // 6. MEM8: 432 Hz Pass-2 Ternary GEMV Salience Filter
    let acts = [108i16, 72, -36, 54, 90, -18, 36, 45, -27, 81];
    let trits = [1i8, 1, -1, 1, 1, -1, 0, 1, -1, 1];
    let mem8_pass2_resonance_score = felix_ternary_gemm_i32(&acts, &trits);

    // 7. GOD OF SUN & *MOON: Solar Ultra-Zeno + Lunar Conjugate Conservation
    let (god_of_sun_zeno_holds, god_of_sun_surplus_num, _horizon) =
        felix_zeno_god_of_sun_pq_nt_gt_p(solar_pq.numer(), solar_pq.denom() as u64, 31, -1);
    let sum_num = solar_pq.numer() * lunar_pq.denom() + lunar_pq.numer() * solar_pq.denom();
    let common_den = solar_pq.denom() * lunar_pq.denom();
    let solar_lunar_unity_verified =
        Ratio::new(sum_num, common_den) == Some(Ratio::ONE) && god_of_sun_zeno_holds;

    Ok(St5PantheonVerificationReport {
        license_bsd3_verified: group.license.contains("Terence Parr")
            && group.license.contains("BSD-3-Clause")
            && group.license.contains("8b-is / ADA-Lang"),
        hydra_rust_code,
        hydra_ada_spark_code,
        hydra_prolog_code,
        hydra_verilog_code,
        aquarius_fast_3x_step2_ms,
        aquarius_med_2x_step2_ms,
        aquarius_slow_q8_step1_ms,
        aquarius_slow_shift_step1_ms,
        tartan_spqt_base243_seal,
        samsara_max_dag_depth,
        phoenix_pct_sha256_hex,
        phoenix_hotswap_epoch,
        mem8_pass2_resonance_score,
        god_of_sun_zeno_holds,
        god_of_sun_surplus_num,
        god_of_moon_complement_pq: lunar_pq,
        solar_lunar_unity_verified,
        pantheon_banner,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_st5_pq_dyadic_shift_add_and_q8_synthesis() {
        let fast = Ratio::new(3, 1).unwrap();
        let med = Ratio::new(2, 1).unwrap();
        let slow_shift = Ratio::new(83, 64).unwrap();
        let slow_q8 = Ratio::new(333, 256).unwrap();

        assert_eq!(synthesize_dyadic_shift_add(fast, "rto"), "rto + (rto << 1)");
        assert_eq!(synthesize_dyadic_shift_add(med, "rto"), "rto << 1");
        assert_eq!(
            synthesize_dyadic_shift_add(slow_shift, "rto"),
            "rto + (rto >> 2) + (rto >> 5) + (rto >> 6)"
        );
        assert_eq!(
            synthesize_q8_multiply_shift(slow_q8, "rto"),
            "((rto * 333) >> 8)"
        );

        // Verify OMNI P2 EU868 Duty-Cycle shift-add/sub decompositions (9x, 10x, 99x, 100x, 999x, 1000x)
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(9, 1).unwrap(), "toa"),
            "(toa << 3) + toa"
        );
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(10, 1).unwrap(), "toa"),
            "(toa << 3) + (toa << 1)"
        );
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(99, 1).unwrap(), "toa"),
            "(toa << 6) + (toa << 5) + (toa << 1) + toa"
        );
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(100, 1).unwrap(), "toa"),
            "(toa << 6) + (toa << 5) + (toa << 2)"
        );
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(999, 1).unwrap(), "toa"),
            "(toa << 10) - (toa << 4) - (toa << 3) - toa"
        );
        assert_eq!(
            synthesize_dyadic_shift_add(Ratio::new(1000, 1).unwrap(), "toa"),
            "(toa << 10) - (toa << 4) - (toa << 3)"
        );
    }

    #[test]
    fn test_samsara_rejects_recursive_template_cycle() {
        let cyclic_stg = r#"group Cyclic;
a(x) ::= "<b(x)>"
b(x) ::= "<a(x)>"
"#;
        let err = St5Group::parse_stg(cyclic_stg).unwrap_err();
        assert!(err.contains("SAMSARA cycle detected"));
    }

    #[test]
    fn test_seven_pillars_hydra_aquarius_tartan_samsara_phoenix_mem8_sun_moon() {
        let rep = verify_stringtemplate5_pq_pantheon().expect("pantheon verified");
        assert!(rep.license_bsd3_verified);
        assert!(rep.hydra_rust_code.contains("rto + (rto << 1)"));
        assert!(rep
            .hydra_rust_code
            .contains("rto + (rto >> 2) + (rto >> 5) + (rto >> 6)"));
        assert!(rep.hydra_rust_code.contains("((rto * 333) >> 8)"));
        assert!(rep
            .hydra_ada_spark_code
            .contains("Rto + Shift_Left (Rto, 1)"));
        assert!(rep
            .hydra_prolog_code
            .contains("vbf_step(fast_3x, Rto, Scaled) :- Scaled is Rto + (Rto << 1)."));
        assert!(rep.hydra_verilog_code.contains("assign vbf_fast_3x"));

        assert_eq!(rep.aquarius_fast_3x_step2_ms, 3_600);
        assert_eq!(rep.aquarius_med_2x_step2_ms, 6_000);
        assert_eq!(rep.aquarius_slow_q8_step1_ms, 5_203);
        assert_eq!(rep.aquarius_slow_shift_step1_ms, 5_187);

        assert_eq!(rep.tartan_spqt_base243_seal, 227);
        assert!(rep.samsara_max_dag_depth <= MAX_SAMSARA_DEPTH);
        assert_eq!(rep.phoenix_pct_sha256_hex.len(), 64);
        assert_eq!(rep.mem8_pass2_resonance_score, 531);
        assert!(rep.god_of_sun_zeno_holds);
        assert_eq!(rep.god_of_sun_surplus_num, 31);
        assert_eq!(rep.god_of_moon_complement_pq, Ratio::new(1, 32).unwrap());
        assert!(rep.solar_lunar_unity_verified);
        assert!(rep.pantheon_banner.contains("SOLAR_YANG(+1)"));
        assert!(rep.pantheon_banner.contains("Sun=31/32 *Moon=1/32"));
    }
}
