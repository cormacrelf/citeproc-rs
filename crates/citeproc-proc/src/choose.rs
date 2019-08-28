// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2018 Corporation for Digital Scholarship

use crate::prelude::*;

use crate::helpers::sequence;
use citeproc_io::DateOrRange;
use csl::style::{Affixes, Choose, Cond, CondSet, Conditions, Else, IfThen, Match};
use std::sync::Arc;

impl<'c, O> Proc<'c, O> for Arc<Choose>
where
    O: OutputFormat,
{
    fn intermediate(
        &self,
        db: &impl IrDatabase,
        state: &mut IrState,
        ctx: &CiteContext<'c, O>,
    ) -> IrSum<O>
    where
        O: OutputFormat,
    {
        // XXX: should you treat conditional evaluations as a "variable test"?
        let Choose(ref head, ref rest, ref last) = **self;
        let mut disamb = false;
        let mut found;
        {
            let BranchEval {
                disambiguate,
                content,
            } = eval_ifthen(head, db, state, ctx);
            found = content;
            disamb = disamb || disambiguate;
        }
        // check the <if> element
        if let Some((content, gv)) = found {
            return if disamb {
                (IR::ConditionalDisamb(self.clone(), Box::new(content)), gv)
            } else {
                (content, gv)
            };
        } else {
            // check the <else-if> elements
            for branch in rest.iter() {
                if found.is_some() {
                    break;
                }
                let BranchEval {
                    disambiguate,
                    content,
                } = eval_ifthen(branch, db, state, ctx);
                found = content;
                disamb = disamb || disambiguate;
            }
        }
        // did any of the <else-if> elements match?
        if let Some((content, gv)) = found {
            if disamb {
                (IR::ConditionalDisamb(self.clone(), Box::new(content)), gv)
            } else {
                (content, gv)
            }
        } else {
            // if not, <else>
            let Else(ref els) = last;
            let (content, gv) = sequence(db, state, ctx, &els, "".into(), None, Affixes::default());
            if disamb {
                (IR::ConditionalDisamb(self.clone(), Box::new(content)), gv)
            } else {
                (content, gv)
            }
        }
    }
}

struct BranchEval<O: OutputFormat> {
    // the bools indicate if disambiguate was set
    disambiguate: bool,
    content: Option<IrSum<O>>,
}

fn eval_ifthen<'c, O>(
    branch: &'c IfThen,
    db: &impl IrDatabase,
    state: &mut IrState,
    ctx: &CiteContext<'c, O>,
) -> BranchEval<O>
where
    O: OutputFormat,
{
    let IfThen(ref conditions, ref elements) = *branch;
    let (matched, disambiguate) = eval_conditions(conditions, ctx, db);
    let content = if matched {
        Some(sequence(
            db,
            state,
            ctx,
            &elements,
            "".into(),
            None,
            Affixes::default(),
        ))
    } else {
        None
    };
    BranchEval {
        disambiguate,
        content,
    }
}

// first bool is the match result
// second bool is disambiguate=true
fn eval_conditions<'c, O>(
    conditions: &'c Conditions,
    ctx: &CiteContext<'c, O>,
    db: &impl IrDatabase,
) -> (bool, bool)
where
    O: OutputFormat,
{
    let Conditions(ref match_type, ref conditions) = *conditions;
    let mut tests = conditions.iter().map(|c| eval_condset(c, ctx, db));
    let disambiguate = conditions.iter().any(|c| {
        c.conds.contains(&Cond::Disambiguate(true)) || c.conds.contains(&Cond::Disambiguate(false))
    }) && ctx.disamb_pass != Some(DisambPass::Conditionals);

    (run_matcher(&mut tests, match_type), disambiguate)
}

fn eval_condset<'c, O>(
    cond_set: &'c CondSet,
    ctx: &CiteContext<'c, O>,
    db: &impl IrDatabase,
) -> bool
where
    O: OutputFormat,
{
    let style = db.style();
    use citeproc_io::DateOrRange;
    use csl::variables::DateVariable;

    let mut iter_all = cond_set.conds.iter().filter_map(|cond| {
        Some(match cond {
            Cond::Variable(var) => ctx.has_variable(*var, db),
            Cond::IsNumeric(var) => ctx.is_numeric(*var, db),
            Cond::Disambiguate(d) => *d == (ctx.disamb_pass == Some(DisambPass::Conditionals)),
            Cond::Type(typ) => ctx.reference.csl_type == *typ,
            Cond::Position(pos) => db.cite_position(ctx.cite.id).0.matches(*pos),

            Cond::HasYearOnly(_) | Cond::HasMonthOrSeason(_) | Cond::HasDay(_)
                if !style.features.condition_date_parts =>
            {
                return None;
            }

            Cond::HasYearOnly(dvar) => ctx
                .reference
                .date
                .get(dvar)
                .map(|dor| match dor {
                    DateOrRange::Single(d) => d.month == 0 && d.day == 0,
                    DateOrRange::Range(d1, d2) => {
                        d1.month == 0 && d1.day == 0 && d2.month == 0 && d2.day == 0
                    }
                    _ => false,
                })
                .unwrap_or(false),
            Cond::HasMonthOrSeason(dvar) => ctx
                .reference
                .date
                .get(dvar)
                .map(|dor| match dor {
                    DateOrRange::Single(d) => d.month != 0,
                    DateOrRange::Range(d1, d2) => {
                        // XXX: is OR the right operator here?
                        d1.month != 0 || d2.month != 0
                    }
                    _ => false,
                })
                .unwrap_or(false),
            Cond::HasDay(dvar) => ctx
                .reference
                .date
                .get(dvar)
                .map(|dor| match dor {
                    DateOrRange::Single(d) => d.day != 0,
                    DateOrRange::Range(d1, d2) => {
                        // XXX: is OR the right operator here?
                        d1.day != 0 || d2.day != 0
                    }
                    _ => false,
                })
                .unwrap_or(false),
            _ => return None,
            // TODO: is_uncertain_date ("ca. 2003"). CSL and CSL-JSON do not specify how this is meant to
            // work.
            // Actually, is_uncertain_date (+ circa) is is a CSL-JSON thing.
        })
    });

    run_matcher(&mut iter_all, &cond_set.match_type)
}

fn run_matcher<I: Iterator<Item = bool>>(bools: &mut I, match_type: &Match) -> bool {
    match *match_type {
        Match::Any => bools.any(|b| b),
        Match::Nand => bools.any(|b| !b),
        Match::All => bools.all(|b| b),
        Match::None => bools.all(|b| !b),
    }
}
