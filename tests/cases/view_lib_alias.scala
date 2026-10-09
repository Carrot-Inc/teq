// jars: view-lib
// A worker asks for `K.Lift` in a body, then the loader's lock holder types a library body that
// names it: the copy of the match alias seen through `K` is the base's, made under the lock over
// the worker's prefix exported, one alias for both. The signature
// below enters `K` and its alias before the fork.
import viewlib.*

def loads(k: K.type): Unit = ()

@main def main(): Unit =
  val lift: K.Lift[Int] = "own"
  println(lift)
  println(Use.lifted)
