# Quent: Query Engine Telemetry

A PoC for (distributed) query engine telemetry.
We aim to build this in one month.

TODO, super rudimentary, simple, easy and quick to duct-tape together poor man's Quenta stack:
- [ ]: Query Engine Model specification
  - See [model.md](./model.md)
- [ ]: Client API
  - This is the thing engines use in their code paths to capture telemetry according to the model.
  - Will write this in Rust with C-style API so it will be easy to create (or generate) bindings to anything (e.g. Python, C, C++, Java, Go).
- [ ]: Distributed transport and collection layer
  - To be able to do this within a VERY short amount of time, we will use OTel logging for now, seem like it has matured enough to be useful.
  - [ ]: Figure out the state of OTel collectors, probably sink logs to a file (hopefully something more efficient than JSON but if that's our only choice, so be it)
- [ ]: Post-processing
  - [ ]: Could just be a Python script parsing and transforming the OTel collector output.
- [ ]: Visualization
  - [ ]: Could just be a Python script.
