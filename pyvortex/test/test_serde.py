#  (c) Copyright 2024 Fulcrum Technologies, Inc. All rights reserved.
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

import pyarrow as pa
from pyarrow import fs

import vortex

local = fs.LocalFileSystem()


def test_serde(tmp_path):
    a = pa.array([0, 1, 2, 3])
    arr = vortex.encode(a)
    assert isinstance(arr, vortex.PrimitiveArray)
    subfs = fs.SubTreeFileSystem(str(tmp_path), local)
    with subfs.open_output_stream("array.enc", buffer_size=8192) as nf:
        vortex.write(arr, nf)

    with subfs.open_input_stream("array.enc", buffer_size=8192) as nf:
        read_array = vortex.read(arr.dtype, nf)
        assert isinstance(read_array, vortex.PrimitiveArray)