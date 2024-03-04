// (c) Copyright 2024 Fulcrum Technologies, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use vortex::array::{
    check_validity_buffer, Array, ArrayRef, ArrowIterator, Encoding, EncodingId, EncodingRef,
};
use vortex::compress::EncodingCompression;
use vortex::dtype::DType;
use vortex::error::VortexResult;
use vortex::formatter::{ArrayDisplay, ArrayFormatter};
use vortex::scalar::{NullableScalar, Scalar};
use vortex::serde::{ArraySerde, EncodingSerde};
use vortex::stats::{Stat, Stats, StatsCompute, StatsSet};

mod compress;
mod serde;

#[derive(Debug, Clone)]
pub struct BitPackedArray {
    encoded: ArrayRef,
    validity: Option<ArrayRef>,
    patches: Option<ArrayRef>,
    len: usize,
    bit_width: usize,
    dtype: DType,
    stats: Arc<RwLock<StatsSet>>,
}

impl BitPackedArray {
    pub fn try_new(
        encoded: ArrayRef,
        validity: Option<ArrayRef>,
        patches: Option<ArrayRef>,
        bit_width: usize,
        dtype: DType,
        len: usize,
    ) -> VortexResult<Self> {
        let validity = validity.filter(|v| !v.is_empty());
        check_validity_buffer(validity.as_ref())?;

        // TODO(ngates): check encoded has type u8

        Ok(Self {
            encoded,
            validity,
            patches,
            bit_width,
            len,
            dtype,
            stats: Arc::new(RwLock::new(StatsSet::new())),
        })
    }

    #[inline]
    pub fn encoded(&self) -> &dyn Array {
        self.encoded.as_ref()
    }

    #[inline]
    pub fn bit_width(&self) -> usize {
        self.bit_width
    }

    #[inline]
    pub fn validity(&self) -> Option<&dyn Array> {
        self.validity.as_deref()
    }

    #[inline]
    pub fn patches(&self) -> Option<&dyn Array> {
        self.patches.as_deref()
    }

    pub fn is_valid(&self, index: usize) -> bool {
        self.validity()
            .map(|v| v.scalar_at(index).and_then(|v| v.try_into()).unwrap())
            .unwrap_or(true)
    }
}

impl Array for BitPackedArray {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn boxed(self) -> ArrayRef {
        Box::new(self)
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn dtype(&self) -> &DType {
        &self.dtype
    }

    #[inline]
    fn stats(&self) -> Stats {
        Stats::new(&self.stats, self)
    }

    fn scalar_at(&self, index: usize) -> VortexResult<Box<dyn Scalar>> {
        if !self.is_valid(index) {
            return Ok(NullableScalar::none(self.dtype().clone()).boxed());
        }

        if let Some(patch) = self
            .patches()
            .and_then(|p| p.scalar_at(index).ok())
            .and_then(|p| p.into_nonnull())
        {
            return Ok(patch);
        }

        todo!("Decode single element from BitPacked array");
    }

    fn iter_arrow(&self) -> Box<ArrowIterator> {
        todo!()
    }

    fn slice(&self, _start: usize, _stop: usize) -> VortexResult<ArrayRef> {
        unimplemented!("BitPackedArray::slice")
    }

    #[inline]
    fn encoding(&self) -> EncodingRef {
        &BitPackedEncoding
    }

    #[inline]
    fn nbytes(&self) -> usize {
        self.encoded().nbytes()
            + self.patches().map(|p| p.nbytes()).unwrap_or(0)
            + self.validity().map(|v| v.nbytes()).unwrap_or(0)
    }

    fn serde(&self) -> &dyn ArraySerde {
        self
    }
}

impl<'arr> AsRef<(dyn Array + 'arr)> for BitPackedArray {
    fn as_ref(&self) -> &(dyn Array + 'arr) {
        self
    }
}

impl ArrayDisplay for BitPackedArray {
    fn fmt(&self, f: &mut ArrayFormatter) -> std::fmt::Result {
        f.writeln(format!("packed: u{}", self.bit_width()))?;
        if let Some(p) = self.patches() {
            f.writeln("patches:")?;
            f.indent(|indent| indent.array(p.as_ref()))?;
        }
        f.array(self.encoded())
    }
}

impl StatsCompute for BitPackedArray {
    fn compute(&self, _stat: &Stat) -> StatsSet {
        // TODO(ngates): implement based on the encoded array
        StatsSet::from(HashMap::new())
    }
}

#[derive(Debug)]
pub struct BitPackedEncoding;

pub const FL_BITPACKED_ENCODING: EncodingId = EncodingId::new("fastlanes.bitpacked");

impl Encoding for BitPackedEncoding {
    fn id(&self) -> &EncodingId {
        &FL_BITPACKED_ENCODING
    }

    fn compression(&self) -> Option<&dyn EncodingCompression> {
        Some(self)
    }

    fn serde(&self) -> Option<&dyn EncodingSerde> {
        Some(self)
    }
}