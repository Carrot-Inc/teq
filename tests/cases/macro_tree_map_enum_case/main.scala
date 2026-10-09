enum T:
  case A
  case B(n: Int)

enum Opt[+X]:
  case Nothing
  case Just(x: X)

case class Box(n: Int)

@main def run =
  println(same(T.B(1)))
  println(same(T.B(1)) == T.B(1))
  println(same(T.B(1)) match
    case T.B(n) => n
    case T.A => 0)
  println(same(Opt.Just("x")))
  println(same(Box(3)))
  println(same(T.A))
