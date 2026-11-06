# Agent instructions

## `quent-open` compatibility

`quent-open` builds a viewer against the Quent revision recorded in an
artifact's `model.qmi`. Existing artifacts must remain openable when public
crates, package names, features, or APIs used by the generated wrapper change.

Before making such a breaking change:

- Add an immutable full commit SHA from `upstream/main` immediately before the
  change as a compatibility boundary.
- Select the complete wrapper contract from Git ancestry before building. Do
  not use failed builds or compiler error text for feature detection.
- Retain the wrapper generation path required by revisions before the boundary.
- Do not move or remove an existing boundary while artifacts may reference
  revisions on either side of it.
- Extend the `quent-open` compatibility test to cover the preserved path.

A predecessor SHA is a valid boundary only when the breaking change is the next
change merged into `upstream/main`. If that ordering cannot be guaranteed, use
the final merge commit or another commit that unambiguously contains the new
contract. With a predecessor boundary, only strict descendants use the new
contract; older and unrelated branch revisions use the preserved contract.

Once releases are available, prefer the first release tag containing a new
wrapper contract as its compatibility boundary. Keep full-SHA boundaries for
artifacts produced from untagged development revisions.
