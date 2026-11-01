enum F(val s: Int):
  case G(override val s: Int) extends F(s)
enum H(val s: Int):
  case K extends H(1)
  case J(t: String, override val s: Int) extends H(s)
enum Changed(val s: Int):
  case Plus(override val s: Int) extends Changed(s + 1)
  case Other(override val s: Int, other: Int) extends Changed(other)
enum Clauses(val s: Int)(val t: String):
  case C(override val s: Int)(override val t: String) extends Clauses(s)(t)
enum Generic[T](val s: T):
  case G[T](override val s: T) extends Generic[T](s)
enum Using(val s: Int)(using val t: String):
  case U(override val s: Int)(using override val t: String) extends Using(s)(using t)
enum UsingOnly(using val s: Int):
  case U(override val s: Int) extends UsingOnly(using s)
trait X
enum WithTrait(val s: Int):
  case W(override val s: Int) extends WithTrait(s) with X
class A(val s: Int)
enum ClassParent(override val s: Int) extends A(s):
  case P(override val s: Int) extends ClassParent(s)
enum Through(s: Int) extends A(s):
  case Q(override val s: Int) extends Through(s)
object O:
  enum N(val s: Int):
    case M(override val s: Int) extends N(s)
def local(): Int =
  enum L(val s: Int):
    case V(override val s: Int) extends L(s)
  L.V(1).s
