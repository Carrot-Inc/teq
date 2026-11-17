// jars: bound-argument-lib
// std: lean scala-library
// targets: js interp jvm
// Named arguments written out of order are evaluated in the order written: the argument written first is
// bound before the object named after it is loaded, an object of a library's classes too, whose load the
// program's build knows nothing of (tests/support/bound_argument_lib.scala: each object's load prints); and
// a right-associative operator's left operand is evaluated before the object on its right is loaded.
import boundlib.*

@main def run(): Unit =
  println("1 an object with an extension after a bound argument"); pair(b = effect("b"), a = Tagged)
  println("2 an object with an export after a bound argument"); pair(b = effect("b"), a = Shared)
  println("3 an object with neither after a bound argument"); pair(b = effect("b"), a = Bare)
  println("4 an object called on after its bound arguments"); Host.take(b = effect("b"), a = effect("a"))
  println("5 an object loaded by its base's secondary constructor after a bound argument"); pair(b = effect("b"), a = Late)
  println("6 a right-associative operator's left operand before the object on its right"); effect("l") :: Cons
  println("7 the same, the object loaded by its base's secondary constructor"); effect("l") :: LateCons
  println("8 an object made with an instance its parent is nested in after a bound argument"); pair(b = effect("b"), a = Enclosed)
  println("9 the same on the right of a right-associative operator"); effect("l") :: EnclosedCons
  println("end")
