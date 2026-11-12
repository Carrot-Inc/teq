// jars: view-lib
// A `Box[Int]` made here before the library's
// macros expand: a deferred inline call's
// signature names `Box[Int]`, and an inline
// call's type argument another. The signature
// below names `Box`.
import viewlib.*

def loads(b: Box[String]): Unit = ()

@main def main(): Unit =
  val early: Box[Int] = Box(0)
  println(early)
  println(Lib.make(41))
  println(Lib.keep(Box(7)))
