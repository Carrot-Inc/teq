// expect: warning: match may not be exhaustive.
// expect: warning: match may not be exhaustive.
// expect: warning: match may not be exhaustive.
// expect: warning: match may not be exhaustive.
// expect: warning: match may not be exhaustive.
// expect: warning: match may not be exhaustive.
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
// expect: It would fail on pattern case: false
// expect: It would fail on pattern case: (false, false)
// expect: It would fail on pattern case: Some(Green)
// expect: It would fail on pattern case: (Red, B)
// expect: It would fail on pattern case: _: String
// expect: It would fail on pattern case: A(_)
