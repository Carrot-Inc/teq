// A top-level value spelt as the wrapper's object of x-y.sc beside it: a member of the package object, whose scope
// keeps the spelling, checked against the root package's scope by the encoded name (Namer.checkNoConflict), so it is
// "already defined" as the wrapper's object, at the value, as scalac 3.8.4 refuses Scala CLI's generated file.
val `x-y_sc` = 7
// expect: Extra.scala:4:5: error: x-y_sc is already defined as object x$minusy_sc in tests/errors/wrapper_value_collision/x-y.sc
