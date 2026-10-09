package app

import lib.{Monoid, Reusability}
import lib.Reusability.MapImplicits.reusabilityMap
import lib.Reusability.MapImplicits.notAGiven
import lib.Reusability.All.given
import lib.given
import model.*

def reuse[A](a: A, b: A)(using r: Reusability[A]): Boolean = r.test(a, b)
def combineAll[A](xs: List[A])(using m: Monoid[A]): A = xs.foldLeft(m.empty)(m.combine)

@main def main(): Unit =
  // extension methods of givens that live in the companion of the receiver's type,
  // of a base type or in the object around it
  println(Color.Red.entryName)
  println(Color.Green.entryNameUpper)
  val color: Color = Color.Green
  println(color.entryName)
  println(Dog("rex").label)
  val animal: Animal = Dog("rex")
  println(animal.label)
  println(Shapes.Square(3).label)
  println(Secret.reveal(Secret(7)))

  // a given imported by name, a plain member imported by name, `given` imports of an object
  // that inherits a given, and of a package
  println(reuse(Map(1 -> 2), Map(1 -> 2)))
  println(reuse(1, 2))
  println(reuse("a", "a"))
  println(reuse(1.5, 1.5))
  println(notAGiven)
  println(reusabilityMap[Int, Int].test(Map(), Map()))
  println(inheritedInt.test(1, 1))
  println(combineAll(List(1, 2, 3)))
  println(intMonoid.empty)
  println(lib.packageHelper)
