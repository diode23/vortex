use arrow2::array::Array as ArrowArray;

use crate::array::binary::{VarBinArray, VarBinViewArray};
use crate::array::bool::BoolArray;
use crate::array::chunked::ChunkedArray;
use crate::array::constant::ConstantArray;
use crate::array::primitive::PrimitiveArray;
use crate::array::ree::REEArray;
use crate::array::struct_::StructArray;
use crate::error::{EncError, EncResult};
use crate::scalar::Scalar;
use crate::types::DType;

pub mod binary;
pub mod bool;
pub mod constant;
pub mod primitive;
pub mod ree;

pub mod chunked;
pub mod encode;
pub mod struct_;

type ArrowIterator = dyn Iterator<Item = Box<dyn ArrowArray>>;

/// An Enc Array is the base object representing all arrays in enc.
///
/// Arrays have a dtype and an encoding. DTypes represent the logical type of the
/// values stored in an pyenc array. Encodings represent the physical layout of the
/// array.
///
/// This differs from Apache Arrow where logical and physical are combined in
/// the data type, e.g. LargeString, RunEndEncoded.
pub trait ArrayEncoding {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn dtype(&self) -> DType;
    fn scalar_at(&self, index: usize) -> EncResult<Box<dyn Scalar>>;
    fn iter_arrow(&self) -> Box<ArrowIterator>;
    fn slice(&self, offset: usize, length: usize) -> EncResult<Array>;

    fn check_slice_bounds(&self, offset: usize, length: usize) -> EncResult<()> {
        if offset > self.len() {
            return Err(EncError::OutOfBounds(offset, 0, self.len()));
        }
        if offset + length > self.len() {
            return Err(EncError::OutOfBounds(offset + length, 0, self.len()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Array {
    Bool(BoolArray),
    Chunked(ChunkedArray),
    Constant(ConstantArray),
    Primitive(PrimitiveArray),
    REE(REEArray),
    Struct(StructArray),
    VarBin(VarBinArray),
    VarBinView(VarBinViewArray),
}

macro_rules! impls_for_array {
    ($variant:tt, $E:ty) => {
        impl From<$E> for Array {
            fn from(arr: $E) -> Self {
                Self::$variant(arr)
            }
        }
    };
}

impls_for_array!(Bool, BoolArray);
impls_for_array!(Chunked, ChunkedArray);
impls_for_array!(Constant, ConstantArray);
impls_for_array!(Primitive, PrimitiveArray);
impls_for_array!(REE, REEArray);
impls_for_array!(Struct, StructArray);
impls_for_array!(VarBin, VarBinArray);
impls_for_array!(VarBinView, VarBinViewArray);

macro_rules! match_each_encoding {
    ($self:expr, | $_:tt $enc:ident | $($body:tt)*) => ({
        macro_rules! __with_enc__ {( $_ $enc:ident ) => ( $($body)* )}
        match $self {
            Array::Bool(enc) => __with_enc__! { enc },
            Array::Chunked(enc) => __with_enc__! { enc },
            Array::Constant(enc) => __with_enc__! { enc },
            Array::Primitive(enc) => __with_enc__! { enc },
            Array::REE(enc) => __with_enc__! { enc },
            Array::Struct(enc) => __with_enc__! { enc },
            Array::VarBin(enc) => __with_enc__! { enc },
            Array::VarBinView(enc) => __with_enc__! { enc },
        }
    })
}

impl ArrayEncoding for Array {
    fn len(&self) -> usize {
        match_each_encoding! { self, |$enc| $enc.len() }
    }

    fn is_empty(&self) -> bool {
        match_each_encoding! { self, |$enc| $enc.is_empty() }
    }

    fn dtype(&self) -> DType {
        match_each_encoding! { self, |$enc| $enc.dtype() }
    }

    fn scalar_at(&self, index: usize) -> EncResult<Box<dyn Scalar>> {
        match_each_encoding! { self, |$enc| $enc.scalar_at(index) }
    }

    fn iter_arrow(&self) -> Box<ArrowIterator> {
        match_each_encoding! { self, |$enc| $enc.iter_arrow() }
    }

    fn slice(&self, offset: usize, length: usize) -> EncResult<Array> {
        match_each_encoding! { self, |$enc| $enc.slice(offset, length) }
    }
}
