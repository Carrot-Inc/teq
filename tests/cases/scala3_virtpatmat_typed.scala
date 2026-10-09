// Adapted from scala3 tests/run/virtpatmat_typed.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App; `: @unchecked` silences the unreachable-case warning scalac gives as well.
object Test {
 def main(args: Array[String]): Unit = ()
 (("foo": Any): @unchecked) match {
   case x: Int => println("FAILED")
   case x: String => println("OK "+ x)
   case x: String => println("FAILED")
 }
}
