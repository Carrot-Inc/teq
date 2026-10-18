// An object and a top-level value of one spelling whose encoding changes it: the object stands in the root package's
// scope under its encoded name, the value in the package object's under its spelling, and Namer.checkNoConflict asks
// the package's scope for the value's mangled name, so the value is "already defined", as scalac 3.8.4 gives it.
object `a-b`
val `a-b` = 7
object Test { def main(args: Array[String]): Unit = println(`a-b`) }
// expect: 5:5: error: a-b is already defined as object a-b
