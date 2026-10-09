// jars: view-lib
// A worker makes its own `Box[Int]` before the loader's lock holder types the library's macros,
// whose quotes the worker then copies: a deferred inline call's signature names the holder's
// `Box[Int]`, and an inline call's type argument (the capture's record, under the product modes)
// another; both are read in the worker's view. The signature below
// enters `Box` before the fork.
import viewlib.*

def loads(b: Box[String]): Unit = ()

@main def main(): Unit =
  val early: Box[Int] = Box(0)
  println(early)
  println(Lib.make(41))
  println(Lib.keep(Box(7)))
