use crate::compress_ctx;
use arrow_array::types::Int64Type;
use arrow_array::{
    ArrayRef as ArrowArrayRef, PrimitiveArray as ArrowPrimitiveArray, RecordBatch,
    RecordBatchReader,
};
use arrow_select::concat::concat_batches;
use arrow_select::take::take_record_batch;
use itertools::Itertools;
use lance::Dataset;
use lance_arrow_array::RecordBatch as LanceRecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;
use vortex::array::chunked::ChunkedArray;
use vortex::array::primitive::PrimitiveArray;
use vortex::array::{ArrayRef, IntoArray};
use vortex::arrow::FromArrowType;
use vortex::compute::flatten::flatten;
use vortex::compute::take::take;
use vortex::ptype::PType;
use vortex::serde::{ReadCtx, WriteCtx};
use vortex_error::VortexResult;
use vortex_schema::DType;

pub fn open_vortex(path: &Path) -> VortexResult<ArrayRef> {
    let mut file = File::open(path)?;
    let dummy_dtype: DType = PType::U8.into();
    let mut read_ctx = ReadCtx::new(&dummy_dtype, &mut file);
    let dtype = read_ctx.dtype()?;
    read_ctx.with_schema(&dtype).read()
}

pub fn compress_vortex<W: Write>(parquet_path: &Path, write: &mut W) -> VortexResult<()> {
    let taxi_pq = File::open(parquet_path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(taxi_pq)?;

    // FIXME(ngates): #157 the compressor should handle batch size.
    let reader = builder.with_batch_size(65_536).build()?;

    let dtype = DType::from_arrow(reader.schema());
    let ctx = compress_ctx();

    let chunks = reader
        .map(|batch_result| batch_result.unwrap())
        .map(|record_batch| {
            let vortex_array = record_batch.into_array();
            ctx.compress(&vortex_array, None).unwrap()
        })
        .collect_vec();
    let chunked = ChunkedArray::new(chunks, dtype.clone());

    let mut write_ctx = WriteCtx::new(write);
    write_ctx.dtype(&dtype).unwrap();
    write_ctx.write(&chunked).unwrap();
    Ok(())
}

pub fn take_vortex(path: &Path, indices: &[u64]) -> VortexResult<ArrayRef> {
    let array = open_vortex(path)?;
    let taken = take(&array, &PrimitiveArray::from(indices.to_vec()))?;
    // For equivalence.... we flatten to make sure we're not cheating too much.
    flatten(&taken).map(|x| x.into_array())
}

pub fn take_parquet(path: &Path, indices: &[u64]) -> VortexResult<RecordBatch> {
    let file = File::open(path)?;

    // TODO(ngates): enable read_page_index
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();

    // We figure out which row groups we need to read and a selection filter for each of them.
    let mut row_groups = HashMap::new();
    let mut row_group_offsets = vec![0];
    row_group_offsets.extend(
        builder
            .metadata()
            .row_groups()
            .iter()
            .map(|rg| rg.num_rows())
            .scan(0i64, |acc, x| {
                *acc += x;
                Some(*acc)
            }),
    );

    for idx in indices {
        let row_group_idx = row_group_offsets
            .binary_search(&(*idx as i64))
            .unwrap_or_else(|e| e - 1);
        row_groups
            .entry(row_group_idx)
            .or_insert_with(Vec::new)
            .push((*idx as i64) - row_group_offsets[row_group_idx]);
    }
    let row_group_indices = row_groups
        .keys()
        .sorted()
        .map(|i| row_groups.get(i).unwrap().clone())
        .collect_vec();

    let reader = builder
        .with_row_groups(row_groups.keys().copied().collect_vec())
        // FIXME(ngates): our indices code assumes the batch size == the row group sizes
        .with_batch_size(10_000_000)
        .build()
        .unwrap();

    let schema = reader.schema();

    let batches = reader
        .into_iter()
        .enumerate()
        .map(|(idx, batch)| {
            let batch = batch.unwrap();
            let indices = ArrowPrimitiveArray::<Int64Type>::from(row_group_indices[idx].clone());
            let indices_array: ArrowArrayRef = Arc::new(indices);
            take_record_batch(&batch, &indices_array).unwrap()
        })
        .collect_vec();

    Ok(concat_batches(&schema, &batches)?)
}

pub fn take_lance(path: &Path, indices: &[u64]) -> LanceRecordBatch {
    Runtime::new()
        .unwrap()
        .block_on(async_take_lance(path, indices))
}

async fn async_take_lance(path: &Path, indices: &[u64]) -> LanceRecordBatch {
    let dataset = Dataset::open(path.to_str().unwrap()).await.unwrap();
    dataset.take(indices, dataset.schema()).await.unwrap()
}