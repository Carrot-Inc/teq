// teq: --wtostring-interpolated --werror
//> using options -Wtostring-interpolated -Werror
// Every binder of a `String | Null` position is a `String`, as dotty's `Typer.typedBind` types it (the type of the
// typed body, simplified by `Typer.simplify`): a case class's pattern over a `Box[String | Null]`, an `unapplySeq`
// and a single `unapply` written here, a binder over a pattern, a `Regex`, a `for` with a pattern. scalac 3.8.4
// compiles this under `-Wtostring-interpolated -Werror`.
final case class Box[A](a: A)
object L2:
  def b: Box[String | Null] = Box("b")
object Mine:
  def unapplySeq(s: String): Option[List[String | Null]] = Some(s.split("-").toList)
object One:
  def unapply(s: String): Option[String | Null] = Some(s)
@main def run(): Unit =
  L2.b match
    case Box(x) => println(s"1 $x")
  val bb: Box[String | Null] = L2.b
  bb match
    case Box(y) => println(s"2 $y")
  "12-34" match
    case Mine(a, b) => println(s"3 $a $b")
    case _ => ()
  "one" match
    case One(o) => println(s"4 $o")
    case _ => ()
  "12-34" match
    case whole @ Mine(a, _) => println(s"5 $whole $a")
    case _ => ()
  val re = "(\\d+)-(\\d+)".r
  "12-34" match
    case re(c, d) => println(s"6 $c $d")
    case _ => ()
  val (p, q) = (L2.b.a, 1)
  println(s"7 $p")
  for case re(e, f) <- List("56-78") do println(s"8 $e $f")
