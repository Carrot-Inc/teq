// The case region in each place a case clause stands: a partial function's braces, a catch's indented
// cases, a match over an enum whose own cases are no case clauses (`enumCase` parses them outside `InCase`),
// a guard continued after a trailing operator, and a for's generators, whose `case` opens no region.
enum S:
  case A, B, C
  case D(n: Int)
def f(s: S, k: Int): Int = s match
  case S.A | S.B |
      S.C if k > 0 => 1
  case S.D(n) if n >
      2 => 2
  case _ => 0
def g(xs: List[Int]): List[String] = xs.collect { case 1 |
    2 => "small"
  case 3 => "three" }
def h(t: => Int): String =
  try { t; "ok" } catch
    case _: IllegalStateException |
        _: IllegalArgumentException => "caught"
def k(xs: List[Option[Int]]): List[Int] =
  for
    case Some(a) <- xs
    b = a + 1
  yield b
@main def run(): Unit =
  println(List(f(S.A, 1), f(S.C, 0), f(S.D(3), 0), f(S.D(1), 0)))
  println((g(List(1, 2, 3, 4)), h(throw new IllegalStateException), h(1)))
  println(k(List(Some(1), None, Some(2))))
