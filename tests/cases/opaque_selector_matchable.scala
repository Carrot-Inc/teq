// teq: --source future
// An opaque type's reference derives from `Matchable` as its bound does: outside its scope, with no bound written,
// `Any`, so an extractor pattern on it warns (`Checking.checkMatchable`); inside, as its alias does; applied, as its
// bound with the arguments applied does (`Wrap[AnyRef]` as `AnyRef`).
object O:
  opaque type Id = Int
  def id(n: Int): Id = n
  def inside(x: Id): Boolean = x match
    case E() => true
    case _ => false
  opaque type Code <: Int = Int
  def code(n: Int): Code = n
  opaque type Wrap[A] <: A = A
  def wrap[A](a: A): Wrap[A] = a

object E:
  def unapply(x: Int): Boolean = x > 0

def outside(x: O.Id): Boolean = x match
  case E() => true
  case _ => false

def bounded(x: O.Code): Boolean = x match
  case E() => true
  case _ => false

object Text:
  def unapply(x: String): Boolean = x.nonEmpty

def applied(x: O.Wrap[AnyRef]): Boolean = x match
  case Text() => true
  case _ => false

@main def main(): Unit =
  println(outside(O.id(1)))
  println(O.inside(O.id(0)))
  println(bounded(O.code(2)))
  println(applied(O.wrap("w")))
