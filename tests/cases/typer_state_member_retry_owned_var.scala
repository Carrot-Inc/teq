// A member whose application fails at its third argument gives way to an extension: the first
// argument's typing, `List(1)` against the member's `List[A]`, constrained the application's
// variable, whose instance stays where the retry sets the application aside (the owner rule),
// so the typing is reused for the extension's `List[Int]`, as scalac's
// cached `List[Int]` is (scalac `6`).
class Box:
  def put[A](xs: List[A], y: A, z: String): Int = 0

extension (b: Box) def put(xs: List[Int], y: Int, z: Int): Int = xs.sum + y + z

@main def run(): Unit =
  println(Box().put(List(1), 2, 3))
