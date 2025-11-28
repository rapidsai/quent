# Quent: Query Engine Telemetry

A PoC for (distributed) query engine telemetry.
We aim to build this in one month.

TODO, super rudimentary, simple, easy and quick to duct-tape together poor man's Quenta stack:
- [ ]: Query Engine Model specification
  - See [model.md](./model.md)
  - It seems infeasible to do the entire model in the time given, but we will iterate from top to bottom as far as we can.
- [ ]: Client API
  - This is the thing engines use in their code paths to capture telemetry according to the model.
  - Will write this in Rust with C-style API so it will be easy to generate bindings to anything (e.g. Python, C, C++, Java, Go).
  - We should aim  to define this in a way that makes it easy to re-use later when we rip out the entire rest of the stuff below.
- [ ]: Distributed transport and collection layer
  - To be able to do this within a VERY short amount of time, we will just get the client API to send simple gRPC messages to a single server that dumps the output to a file. This is not decentralized / scalable. The messages will just be serialized records.
  - This is absolutely the first thing that should be replaced later.
  - [ ]: Figure out the state of OTel collectors, probably sink logs to a file (hopefully something more efficient than JSON but if that's our only choice, so be it)
- [ ]: Post-processing
  - [ ]: Could just be a Python script parsing and transforming the OTel collector output.
- [ ]: Visualization
  - [ ]: Could just be a Python script.
