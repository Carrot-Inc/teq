// expect: warning: match may not be exhaustive; missing: false
// expect: warning: match may not be exhaustive; missing: (false, false)
// expect: warning: match may not be exhaustive; missing: Some(Green)
// expect: warning: match may not be exhaustive; missing: (Red, B)
// expect: warning: match may not be exhaustive; missing: String
// expect: warning: match may not be exhaustive; missing: A(_)
// expect: 6 warnings found, errors under --werror
// teq: --werror
enum Color:
  case Red, Green

sealed trait S
case class A(x: Int) extends S
case object B extends S

@main def run(): Unit =
  val b = true
  b match
    case true => println("t")
  val t: (Boolean, Boolean) = (true, false)
  t match
    case (true, _) => println(1)
    case (_, true) => println(2)
  val o: Option[Color] = Some(Color.Red)
  o match
    case Some(Color.Red) => println("r")
    case None => println("n")
  val p: (Color, S) = (Color.Red, B)
  p match
    case (Color.Red, A(_)) => println(1)
    case (Color.Green, _) => println(2)
  val u: Int | String = 1
  u match
    case i: Int => println(i)
  val s: S = B
  s match
    case A(1) => println("one")
    case B => println("b")
