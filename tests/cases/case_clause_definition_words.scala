// A `case` opens a case clause's region unless the token after it is `class` or `object` (dotty's scanner
// fuses `CASECLASS` and `CASEOBJECT`): a variable pattern named `classy` or `objectX` is a clause's, and a
// comment between `case` and `class` still makes a case class.
case /* a definition */ class Box(n: Int)
case object Unit1
def f(x: Any): String = x match
  case classy: Int if classy >
      0 => s"int $classy"
  case objectX: String => s"string $objectX"
  case Box(1) |
      Box(2) => "small box"
  case _ => "other"
@main def run(): Unit = println(List(f(1), f("s"), f(Box(2)), f(Unit1)))
