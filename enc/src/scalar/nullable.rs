use crate::error::EncResult;
use crate::scalar::Scalar;
use crate::types::DType;
use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub enum NullableScalar {
    Some(Box<dyn Scalar>),
    None(DType),
}

impl NullableScalar {
    pub fn some(scalar: Box<dyn Scalar>) -> Self {
        Self::Some(scalar)
    }

    pub fn none(dtype: DType) -> Self {
        Self::None(dtype)
    }
}

impl Scalar for NullableScalar {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        todo!()
    }

    #[inline]
    fn boxed(self) -> Box<dyn Scalar> {
        Box::new(self)
    }
    #[inline]
    fn dtype(&self) -> &DType {
        match self {
            Self::Some(scalar) => scalar.dtype(),
            Self::None(dtype) => dtype,
        }
    }

    fn cast(&self, _dtype: &DType) -> EncResult<Box<dyn Scalar>> {
        todo!()
    }
}
