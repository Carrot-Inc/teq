// expect: 6:9: error: not found: missing
// absent: is not a member of
// expect: 1 error found
// A name not found is one error: the selections on it (`.foo.bar`) report nothing more, the error
// type they are taken on absorbing them as dotty's `ErrorType` does.
val x = missing.foo.bar
