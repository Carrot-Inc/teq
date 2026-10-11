//> using platform js
//> using jsVersion 1.22.0

import scala.scalajs.js

// A catch of `Throwable` over values JavaScript throws that are no `Throwable`: Scala.js wraps each in a
// `js.JavaScriptException` before the cases (`e instanceof Throwable ? e : new JavaScriptException(e)`),
// whose test of a caught `Throwable` is then a null test that always passes, as dotty's `testNotNull`
// for a scrutinee whose type conforms (tests/cases/type_test_null_shapes, the other shapes).
def thrown(code: String, message: Boolean = true): String =
  try { js.eval(code); "none" }
  catch { case e: Throwable => "caught " + e.getClass.getSimpleName + (if message then " " + e.getMessage else "") }
@main def run(): Unit =
  println(thrown("(function(){ throw 42; })()"))
  println(thrown("(function(){ throw 'str'; })()"))
  println(thrown("(function(){ throw { a: 1 }; })()", message = false))
  println(thrown("1"))
