// A package defined in source alone compares its members' names as spelt: only the root and the empty package,
// which dotty's classpath loads with a PackageScope, find a class, an object or a package by either spelling. scalac
// 3.8.4 refuses the other spelling inside a named package, selected from it, through a nested package and imported.
package q$minusr:
  object `c-d`:
    val v = 1
  class `k-l`
  object User:
    def run(): Unit = println(c$minusd.v)
package outer:
  package `x-y`:
    object Z
object Main:
  import q$minusr.c$minusd
  def main(args: Array[String]): Unit =
    println(q$minusr.c$minusd.v)
    println(new q$minusr.k$minusl)
    println(outer.x$minusy.Z)
// expect: 9:31: error: not found: c$minusd
// expect: 14:19: error: c$minusd is not a member of the package
// expect: 16:13: error: value c$minusd is not a member of q$minusr
// expect: 17:17: error: type k$minusl not found
// expect: 18:13: error: value x$minusy is not a member of outer
// expect: 5 errors found
