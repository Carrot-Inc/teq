// A value spelt as an object's encoded name, and one spelt as the source name of an object written encoded: each is
// the package object's member under its spelling, and the root package's scope holds the object under the encoded
// name the value's spelling mangles to (Namer.checkNoConflict), so each value is "already defined", as scalac 3.8.4
// gives them one by one.
object `a-b` { val value = 42 }
val a$minusb = 7
object c$minusd { val value = 42 }
val `c-d` = 7
// expect: 6:5: error: a$minusb is already defined as object a-b
// expect: 8:5: error: c-d is already defined as object c$minusd
