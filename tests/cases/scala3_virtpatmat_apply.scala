// Adapted from scala3 tests/run/virtpatmat_apply.scala (Apache-2.0, see tests/scala3/README.md); replaced: exhaustivity warning silenced with @unchecked, extends App.
object Test {
 def main(args: Array[String]): Unit = ()
 (List(1, 2, 3): @unchecked) match {
   case Nil => println("FAIL")
   case x :: y :: xs if xs.length == 2 => println("FAIL")
   case x :: y :: xs if xs.length == 1 => println("OK "+ y)
 }
}
