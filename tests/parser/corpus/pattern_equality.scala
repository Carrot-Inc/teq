// Pattern equality is `pattern == scrutinee`: cooperative between the numeric types on an
// untyped scrutinee, by the pattern's `equals` for stable identifiers, by identity for `x.type`.
case class Point(x: Int, y: Int)
object Consts:
  val Origin = Point(0, 0)
  val Name = "n" + "ame"
  val Big = 1L << 40
  val One = 1

class Sym(val name: String):
  override def equals(other: Any) = other match
    case s: Sym => name == s.name
    case _ => false

object Syms:
  val a = new Sym("a")
  val other = new Sym("a")

def literals(x: Any): String = x match
  case 1 => "int 1"
  case 1L => "long 1"
  case 2.5 => "double 2.5"
  case 'a' => "char a"
  case true => "true"
  case _ => "other"

def litLong(x: Long): String = x match
  case 1 => "one"
  case 2L => "two"
  case _ => "other"

def litChar(x: Char): String = x match
  case 'a' => "a"
  case 98 => "b via 98"
  case _ => "other"

def litInt(x: Int): String = x match
  case 'a' => "97 via 'a'"
  case _ => "other"

def stable(x: Any): String = x match
  case Consts.Origin => "origin"
  case Consts.Name => "name"
  case Consts.Big => "big"
  case Consts.One => "one"
  case Syms.a => "sym a"
  case _ => "other"

def singleton(s: Sym): Int = s match
  case x: Syms.a.type => 1
  case Syms.a => 2
  case _ => 3

@main def run(): Unit =
  println(List[Any](1, 1L, 1.5, 2.5, 'a', true, 2).map(literals))
  println(litLong(1L) + " " + litLong(2L) + " " + litLong(3L))
  println(litChar('a') + " " + litChar('b') + " " + litChar('c'))
  println(litInt(97) + " " + litInt(98))
  println(List[Any](Point(0, 0), "name", 1L << 40, 1, 1L, Syms.other, Point(1, 0)).map(stable))
  println(singleton(Syms.a) + " " + singleton(Syms.other) + " " + singleton(new Sym("b")))
