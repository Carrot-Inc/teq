// teq: --werror
// A lambda whose body is a match, given where a partial function is expected, is one: its
// cases decide `isDefinedAt` whatever the scrutinee, so `collect` skips what no case takes
// and the match is not checked for exhaustiveness; and a trailing wildcard case on a tuple
// whose cases cover it only by splitting its fields is left alone, as dotc's reachability
// check, which does not split a product, leaves it (it reports a wildcard after `Some(_)` and
// `None` on an `Option`, reachable through `null` alone).
sealed trait Payload
object Payload:
  final case class Post(addr: String) extends Payload
  case object Desk extends Payload
final case class Loan(payload: Payload, member: Option[String])

def posted(loans: List[Loan]): List[(String, String)] =
  loans.collect { loan =>
    (loan.payload, loan.member) match
      case (p: Payload.Post, Some(c)) => (p.addr, c)
  }

def braced(loans: List[Loan]): List[String] =
  loans.collect { loan => { loan.payload match { case Payload.Desk => "desk" } } }

def guarded(xs: List[Int]): List[Int] = xs.collect { x => x match { case n if n > 2 => n * 10 } }

final case class SP(n: Int)
final case class PR(n: Int)
type Joined = (Option[SP], Option[PR], Option[Int])
def read(j: Joined): String = j match
  case (Some(v), _, _) => "sp" + v.n
  case (_, Some(v), _) => "pr" + v.n
  case (_, _, Some(n)) => "n" + n
  case (None, None, None) => "none"
  case _ => throw new Error("more than one")

@main def run(): Unit =
  val loans = List(Loan(Payload.Post("a"), Some("c1")), Loan(Payload.Desk, Some("c2")), Loan(Payload.Post("b"), None))
  println(posted(loans))
  println(braced(loans))
  println(guarded(List(1, 2, 3, 4)))
  println(read((Some(SP(1)), None, None)) + read((None, None, Some(2))) + read((None, None, None)))
