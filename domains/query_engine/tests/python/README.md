# Query Engine Python Integration Test

This test exposes the query engine domain model as a Python extension module
generated with PyO3. It mirrors the C++ integration test under
`domains/query_engine/tests/cpp`.

```bash
cd domains/query_engine/tests/python
maturin develop
python test_query_engine.py
```
