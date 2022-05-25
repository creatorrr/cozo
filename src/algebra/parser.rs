use crate::algebra::op::{
    build_from_clause, Insertion, RelationFromValues, RelationalAlgebra, TaggedInsertion,
    NAME_FROM, NAME_INSERTION, NAME_RELATION_FROM_VALUES, NAME_TAGGED_INSERTION,
    NAME_TAGGED_UPSERT, NAME_UPSERT,
};
use crate::context::TempDbContext;
use crate::data::tuple::OwnTuple;
use crate::data::tuple_set::TableId;
use crate::data::value::StaticValue;
use crate::parser::{Pair, Rule};
use anyhow::Result;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub(crate) enum AlgebraParseError {
    #[error("{0} cannot be chained")]
    Unchainable(String),

    #[error("wrong argument type for {0}({1}): {2}")]
    WrongArgumentType(String, usize, String),

    #[error("Table not found {0}")]
    TableNotFound(String),

    #[error("Wrong table kind {0:?}")]
    WrongTableKind(TableId),

    #[error("Table id not found {0:?}")]
    TableIdNotFound(TableId),

    #[error("Not enough arguments for {0}")]
    NotEnoughArguments(String),

    #[error("Value error {0:?}")]
    ValueError(StaticValue),

    #[error("Parse error {0}")]
    Parse(String),

    #[error("Data key conflict {0:?}")]
    KeyConflict(OwnTuple),

    #[error("No association between {0} and {1}")]
    NoAssociation(String, String),
}

pub(crate) fn assert_rule(pair: &Pair, rule: Rule, name: &str, u: usize) -> Result<()> {
    if pair.as_rule() == rule {
        Ok(())
    } else {
        Err(AlgebraParseError::WrongArgumentType(
            name.to_string(),
            u,
            format!("{:?}", pair.as_rule()),
        )
        .into())
    }
}

pub(crate) fn build_relational_expr<'a>(
    ctx: &'a TempDbContext,
    pair: Pair,
) -> Result<Arc<dyn RelationalAlgebra + 'a>> {
    let mut built: Option<Arc<dyn RelationalAlgebra>> = None;
    for pair in pair.into_inner() {
        let mut pairs = pair.into_inner();
        match pairs.next().unwrap().as_str() {
            NAME_INSERTION => built = Some(Arc::new(Insertion::build(ctx, built, pairs, false)?)),
            NAME_UPSERT => built = Some(Arc::new(Insertion::build(ctx, built, pairs, true)?)),
            NAME_TAGGED_INSERTION => {
                built = Some(Arc::new(TaggedInsertion::build(ctx, built, pairs, false)?))
            }
            NAME_TAGGED_UPSERT => {
                built = Some(Arc::new(TaggedInsertion::build(ctx, built, pairs, true)?))
            }
            NAME_RELATION_FROM_VALUES => {
                built = Some(Arc::new(RelationFromValues::build(ctx, built, pairs)?));
            }
            NAME_FROM => {
                built = Some(build_from_clause(ctx, built, pairs)?);
            }
            _ => unimplemented!(),
        }
    }
    Ok(built.unwrap())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::data::tuple::Tuple;
    use crate::parser::{CozoParser, Rule};
    use crate::runtime::options::default_read_options;
    use crate::runtime::session::tests::create_test_db;
    use anyhow::Result;
    use pest::Parser;
    use std::collections::BTreeMap;
    use std::time::Instant;

    const HR_DATA: &str = include_str!("../../test_data/hr.json");

    #[test]
    fn parse_ra() -> Result<()> {
        let (db, mut sess) = create_test_db("_test_parser.db");
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
                           Values(v: [id, name], [[100, 'confidential'], [101, 'top secret']])
                          .Upsert(Department, d: {...v})
                          "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(ra.get_values()?);
            ctx.txn.commit().unwrap();
        }
        {
            let ctx = sess.temp_ctx(true);
            let s = format!("UpsertTagged({})", HR_DATA);
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, &s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            // for t in ra.iter().unwrap() {
            //     dbg!(t.unwrap());
            // }
            dbg!(ra.get_values()?);

            ctx.txn.commit().unwrap();
        }
        let duration = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = "From(e:HasDependent)";
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(ra.get_values()?);
        }
        let duration2 = start.elapsed();
        let start = Instant::now();
        let mut r_opts = default_read_options();
        r_opts.set_total_order_seek(true);
        r_opts.set_prefix_same_as_start(false);
        let it = sess.main.iterator(&r_opts);
        it.to_first();
        let mut n: BTreeMap<u32, usize> = BTreeMap::new();
        while it.is_valid() {
            let (k, v) = it.pair().unwrap();
            let k = Tuple::new(k);
            let v = Tuple::new(v);
            if v.get_prefix() == 0 {
                *n.entry(k.get_prefix()).or_default() += 1;
            }
            it.next();
        }
        let duration3 = start.elapsed();
        dbg!(duration, duration2, duration3, n);
        Ok(())
    }
}
