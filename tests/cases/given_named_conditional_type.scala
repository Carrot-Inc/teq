// A named given's condition written as a type before `=>` is a using clause of that one parameter, as for an
// anonymous one (dotty's `Parsers.newTermParamssAndParents`: `typesToParams`), after type parameters or alone:
// `given listOrd: [A] => Ord[A] => Ord[List[A]]` is an `Ord[List[A]]` given taking an `Ord[A]`, and
// `given f: Int => String` a String given taking an Int, not a given of function type. scalac 3.8.4 prints
// `list of int`, `s7` and `list of list of int`.
trait Ord[A] { def tag: String }
given Ord[Int] with { def tag = "int" }
given listOrd: [A] => Ord[A] => Ord[List[A]] = new Ord[List[A]] { def tag = "list of " + summon[Ord[A]].tag }
given Int = 7
given f: Int => String = "s" + summon[Int]
@main def run(): Unit =
  println(summon[Ord[List[Int]]].tag)
  println(summon[String])
  println(listOrd[List[Int]].tag)
