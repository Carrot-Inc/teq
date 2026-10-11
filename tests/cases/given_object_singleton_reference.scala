// A stable candidate found for a target that holds a path is its reference, dotty's
// `TermRef(receiver, member)`: the summoned value is `B.u`, a given object's module class
// conforming to its path as an object's does (`TypeComparer.compareNamed`), and a union
// with a path (`B.x.type | Null`) takes the reference too.
trait Token
trait A:
  given u: Token with
    def name = "u"
  given x: String = "ok"
object B extends A
given top: Token with {}
@main def run(): Unit =
  val selected: B.u.type = summon[B.u.type]
  println(selected eq B.u)
  val inferred = summon[B.u.type]
  val again: B.u.type = inferred
  println(again.name)
  println(summon[B.x.type | Null])
  println(summon[B.u.type & Token] eq B.u)
  val t: top.type = summon[top.type]
  println(t eq top)
  // A plain target beside a given object: the one given of its type, no second candidate.
  println(summon[Token] eq top)
