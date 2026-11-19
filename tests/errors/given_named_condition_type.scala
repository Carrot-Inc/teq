// A named given's condition written as a type before `=>` is a using clause (dotty's
// `Parsers.newTermParamssAndParents`): `given listOrd: Ord[Int] => Ord[List[Int]]` is an
// `Ord[List[Int]]` given taking an `Ord[Int]`, so a function on its right side is a type mismatch
// (scalac: E007, found the function's type, required `Ord[List[Int]]`), not a given of function type.
trait Ord[A] { def tag: String }
val mk: Ord[Int] => Ord[List[Int]] = o => new Ord[List[Int]] { def tag = "list of " + o.tag }
given listOrd: Ord[Int] => Ord[List[Int]] = mk
@main def run(): Unit = ()
// expect: 7:45: error: Found:    Ord[Int] => Ord[List[Int]]
// expect+ Required: Ord[List[Int]]
