// expect: 30:24: error: missing argument for parameter rgb
// expect: 31:29: error: type mismatch: found String, required Int
// expect: 32:32: error: illegal reference to fallback from an enum case; write Rgb.fallback
// expect: 33:31: error: only the enum itself takes constructor arguments
// expect: 34:24: error: too many arguments: expected 1
// expect: error: no given instance of type Show[Boolean] was found for parameter evidence$
// expect: 42:36: warning: match may not be exhaustive; missing: Blue
// expect: 46:38: warning: match may not be exhaustive; missing: LParen, Ident(_)
// expect: 50:43: warning: match may not be exhaustive; missing: Flag
// expect: 54:13: error: type mismatch: found String, required T
// expect: 57:22: error: value rgb is not a member of Color
// expect: 65:31: warning: match may not be exhaustive; missing: Loose
// expect: 8 errors found

trait Marker
trait Show[T]:
  def show(t: T): String
given Show[Int] with
  def show(t: Int): String = t.toString

enum Color:
  case Red, Green, Blue

enum Token:
  case Plus, LParen
  case Number(value: Int)
  case Ident(name: String)

enum Rgb(val rgb: Int):
  case Missing extends Rgb
  case Mistyped extends Rgb("red")
  case Unqualified extends Rgb(fallback)
  case Marked extends Rgb(1), Marker(2)
  case TooMany extends Rgb(1, 2)
object Rgb:
  val fallback = 0

enum Key[T: Show](val name: String):
  case Count extends Key[Int]("count")
  case Flag extends Key[Boolean]("flag")

def incomplete(c: Color): String = c match
  case Color.Red => "r"
  case Color.Green => "g"

def incompleteToken(t: Token): Int = t match
  case Token.Plus => 0
  case Token.Number(v) => v

def incompleteKey[T](k: Key[T]): String = k match
  case Key.Count => "count"

def unrefined[T](k: Key[T], fallback: T): T = k match
  case _ => "text"

def notWidened: Int =
  val c = Color.Red; c.rgb

trait Sized
enum Item:
  case Loose
  case Box(n: Int) extends Item, Sized
  case Crate(n: Int) extends Item, Sized

def bySize(i: Item): String = i match
  case s: Sized => "sized"
