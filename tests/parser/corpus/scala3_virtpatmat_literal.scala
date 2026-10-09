// Adapted from scala3 tests/run/virtpatmat_literal.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
 def main(args: Array[String]): Unit = ()
 val a = 1
 1 match {
   case 2 => println("FAILED")
   case 1 => println("OK")
   case `a` => println("FAILED")
 }

 val one = 1
 1 match {
   case 2 => println("FAILED")
   case `one` => println("OK")
   case 1 => println("FAILED")
 }

 1 match {
   case 2 => println("FAILED")
   case Test.one => println("OK")
   case 1 => println("FAILED")
 }

}
