// A class and a top-level type alias of one spelling whose encoding changes it: the alias, the package object's, is
// "already defined" as the class the root package keys by the encoded name (Namer.checkNoConflict), as scalac 3.8.4
// gives it.
class `a-b`
type `a-b` = String
object Test { def main(args: Array[String]): Unit = println("ran") }
// expect: 5:6: error: a-b is already defined as class a-b
