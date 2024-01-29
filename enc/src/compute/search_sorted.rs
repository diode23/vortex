use crate::array::Array;
use crate::error::EncResult;
use crate::polars::IntoPolarsSeries;
use crate::polars::IntoPolarsValue;
use crate::scalar::Scalar;
use polars_core::prelude::*;
use polars_ops::prelude::*;

pub enum SearchSortedSide {
    Left,
    Right,
}

impl From<SearchSortedSide> for polars_ops::prelude::SearchSortedSide {
    fn from(side: SearchSortedSide) -> Self {
        match side {
            SearchSortedSide::Left => polars_ops::prelude::SearchSortedSide::Left,
            SearchSortedSide::Right => polars_ops::prelude::SearchSortedSide::Right,
        }
    }
}

pub fn search_sorted_usize<T: AsRef<dyn Array>>(
    indices: &T,
    index: usize,
    side: SearchSortedSide,
) -> EncResult<usize> {
    let enc_scalar: Box<dyn Scalar> = index.into();
    // Convert index into correctly typed Arrow scalar.
    let enc_scalar = enc_scalar.cast(indices.as_ref().dtype())?;

    let series: Series = indices.as_ref().iter_arrow().into_polars();
    Ok(search_sorted(
        &series,
        &Series::from_any_values("needle", &[enc_scalar.into_polars()], true)?,
        side.into(),
        false,
    )?
    .get(0)
    .unwrap() as usize)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::array::ArrayRef;

    #[test]
    fn test_searchsorted_scalar() {
        let haystack: ArrayRef = vec![1, 2, 3].into();

        assert_eq!(
            search_sorted_usize(&haystack, 0, SearchSortedSide::Left).unwrap(),
            0
        );
        assert_eq!(
            search_sorted_usize(&haystack, 1, SearchSortedSide::Left).unwrap(),
            0
        );
        assert_eq!(
            search_sorted_usize(&haystack, 1, SearchSortedSide::Right).unwrap(),
            1
        );
        assert_eq!(
            search_sorted_usize(&haystack, 4, SearchSortedSide::Left).unwrap(),
            3
        );
    }
}
