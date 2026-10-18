// Only a package's own members, its classes, objects and packages, are found by either spelling of their names
// (dotty's SymbolLoaders.PackageScope): a top-level def is the package object's member, and an object's members
// and nested objects, like a local, are found as spelt, as scalac 3.8.4 refuses the encoded references.
object O:
  val `p-q` = "member"
  object `i-j`:
    val v = "nested"
def `f-g` = "toplevel def"
object Main:
  def main(args: Array[String]): Unit =
    val `x+y` = 3
    println(x$plusy)
    println(f$minusg)
    println(O.i$minusj.v)
    println(O.p$minusq)
// expect: 12:13: error: not found: x$plusy
// expect: 13:13: error: not found: f$minusg
// expect: 14:13: error: value i$minusj is not a member of O
// expect: 15:13: error: value p$minusq is not a member of O
// expect: 4 errors found
