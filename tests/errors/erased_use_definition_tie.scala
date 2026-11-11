import scala.compiletime.erasedValue
inline def broken: Any =
  class Local:
    def first: Int = erasedValue[Int]
    def second: String = erasedValue[String]
  new Local
val result = broken
// scalac 3.8.4 reports both errors at the call, with inline origins at lines 4 then 5.
// expect: 7:14: error: method erasedValue is declared as `erased`, but is in fact used
// expect: inlined from tests/errors/erased_use_definition_tie.scala:4
// expect: inlined from tests/errors/erased_use_definition_tie.scala:5
// expect: 2 errors found
