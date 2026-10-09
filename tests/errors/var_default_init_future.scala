// Under `scala.language.future` the deprecated `= _` of a field is an error (scala3's
// neg/uninitialized-future).
// expect: `= _` has been deprecated; use `= uninitialized` instead.
import scala.language.future
import scala.compiletime.uninitialized

class Foo:
  var a: Int = _
  var b: Int = uninitialized
