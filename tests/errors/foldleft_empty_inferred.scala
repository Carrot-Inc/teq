// The accumulator `foldLeft(List.empty)` infers is a `List[Any]` (foldleft_empty_lambda), as scalac's.
def a1: List[Int] = { val a = List(1, 2).foldLeft(List.empty)((acc, x) => x :: acc); a }
def b1: Vector[Int] = { val b = List(1).foldLeft(Vector.empty)((acc, x) => acc :+ x); b }
// expect: 2:86: error: type mismatch: found List[Any], required List[Int]
// expect: 3:87: error: type mismatch: found Vector[Any], required Vector[Int]
