// The value first and the object after: dotty enters the package object's members after the package's own, so the
// conflict is the value's wherever it stands in the source (Namer.checkNoConflict), as scalac 3.8.4 gives it.
val `a-b` = 7
object `a-b`
object Test { def main(args: Array[String]): Unit = println(`a-b`) }
// expect: 3:5: error: a-b is already defined as object a-b
