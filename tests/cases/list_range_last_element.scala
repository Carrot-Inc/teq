// `List.range` ends on its last element, computed apart as `NumericRange.count` does, so a step
// past `Int.MaxValue` ends the range where adding would wrap; empty and downward ranges, a
// `Long` range across zero and a zero step as scala-library's.
@main def main(): Unit =
  println(List.range(Int.MaxValue - 1, Int.MaxValue, 2))
  println(List.range(Int.MinValue + 1, Int.MinValue, -2))
  println(Vector.range(Int.MaxValue - 4, Int.MaxValue, 3))
  println(List.range(Long.MaxValue - 3L, Long.MaxValue, 2L))
  println(List.range(-5L, 6L, 4L))
  println(List.range(5, 5, 1))
  println(List.range(3, 0, 1))
  println(List.range(10, 0, -3))
  println(try List.range(0, 3, 0).toString catch case e: IllegalArgumentException => e.getMessage)
