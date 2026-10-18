// An object whose encoded name is the wrapper's of x-y.sc beside it: one entry of the root package (dotty's
// PackageScope keys by name.mangled), so the wrapper's object is a duplicate (checkNoConflict, E161), as scalac 3.8.4
// refuses Scala CLI's generated file.
object `x-y_sc` { val answer = 42 }
// expect: x-y.sc:9:8: error: x$minusy_sc is already defined as object x-y_sc in tests/errors/wrapper_object_collision/Extra.scala
// expect: 1 error found
