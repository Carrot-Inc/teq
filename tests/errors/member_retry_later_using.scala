// expect: 11:19: error: no given instance of type Missing was found for parameter x$3
// absent: required String
// A try of a member's retry is the application of the member's argument list (dotty's
// `tryWithImplicitOnQualifier` commits `simpleApply`): the extension that takes the list takes over,
// and its using clause after the list, resolved after it (`adaptNoArgsImplicitMethod`), fails as the
// extension's, scalac's error, where the member's mismatch stood.
class Missing
class B { def f(x: String): Int = 0 }
extension (b: B) def f(x: Int)(using Missing): Int = 42
@main def run(): Unit =
  println(B().f(1))
