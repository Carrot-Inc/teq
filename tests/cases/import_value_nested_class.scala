// Classes nested in the class of a value, named without a prefix through an import of the value's
// members (reactivemongo's `aggregateList(..) { framework => import framework.*; .. }`): made with
// `new` and through their companions, matched by their patterns, with the value as the enclosing
// instance.
trait Framework:
  def tag: String
  case class Match(q: Int):
    override def toString = s"$tag:Match($q)"
  object Limit:
    def apply(n: Int) = s"$tag:limit $n"
  class Project(val x: Int):
    override def toString = s"$tag:Project($x)"
  sealed trait Op
  case class Sort(f: String) extends Op:
    override def toString = s"$tag:Sort($f)"
  case class Pair(a: Int, b: Int):
    override def toString = s"$tag:Pair($a,$b)"
  object Pair

class Coll(name: String):
  object framework extends Framework:
    def tag = name
  def aggregate[A](f: Framework => A): A = f(framework)

def run(fw: Framework) =
  import fw.*
  val ops: List[Op] = List(Sort("a"))
  val m = Match(1)
  val q = m match
    case Match(q) => q
  val pr = Pair(2, 3) match
    case Pair(a, b) => a + b
  (m, Limit(2), new Project(3), Project(4), ops, q, pr, Pair(5, 6))

object Names:
  val Project = 5

// Outside an application a constructor proxy gives way to the binding it shadows.
def shadowed(fw: Framework) =
  import Names.*
  locally:
    import fw.*
    (Project + 1, Project(2))

@main def m() =
  val c = Coll("c")
  println(run(c.framework))
  println(shadowed(c.framework))
  import c.framework.*
  println(Match(5))
  println(new Project(6))
  println(Coll("d").aggregate { framework => import framework.*; List(Match(7), Sort("b")) })
  import c.framework.{Project as P}
  println(P(8))
  println(new P(9))
