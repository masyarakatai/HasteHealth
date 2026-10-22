use super::{
    ClauseTarget, SqlClause, SqlParam, bind, or_predicates, target_params, value_expr,
    wrap_predicate,
};
use crate::query::{
    QueryBuildError,
    value::{UriValues, uri_ancestors},
};

/// Matches any supplied URI: exactly, or by hierarchy for `:above`/`:below`.
pub fn uri_clause(
    values: &UriValues<'_>,
    target: &ClauseTarget,
) -> Result<SqlClause, QueryBuildError> {
    let column = value_expr(target)?;
    let (predicate, params) = match values {
        UriValues::Uris(uris) => or_predicates(uris, target_params(target), |value, params| {
            let (params, i) = bind(params, [SqlParam::Text(value.clone())]);
            Ok((format!("{column} = ${i}"), params))
        })?,
        UriValues::Below(uris) => or_predicates(uris, target_params(target), |value, params| {
            let (params, i) = bind(params, [SqlParam::Text(like_prefix(value))]);
            Ok((format!("{column} LIKE ${i}"), params))
        })?,
        UriValues::Above(uris) => {
            or_predicates(uris, target_params(target), |value, mut params| {
                let first = params.len() + 1;
                let ancestors = uri_ancestors(value);
                let placeholders = (first..first + ancestors.len())
                    .map(|i| format!("${i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                params.extend(ancestors.into_iter().map(|a| SqlParam::Text(a.to_owned())));
                Ok((format!("{column} IN ({placeholders})"), params))
            })?
        }
    };

    Ok(wrap_predicate(target, false, &predicate, params))
}

/// A `LIKE` pattern matching strings that start with `value`.
fn like_prefix(value: &str) -> String {
    let mut pattern = String::with_capacity(value.len() + 1);
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::like_prefix;

    #[test]
    fn like_prefix_escapes_wildcards() {
        assert_eq!(like_prefix("http://a/b_c%d\\"), "http://a/b\\_c\\%d\\\\%");
    }
}
