// expect: 20:34: error: type mismatch: found String, required A
// expect: 20:39: error: type mismatch: found String, required A
// expect: 29:27: error: type mismatch: found String, required A
// expect: 31:38: error: type mismatch: found String, required A
// expect: 4 errors found

// A case class's companion `apply` takes the class's whole declared parameter-list structure
// (`Desugar.classDef`), its types seen from the instance the companion is selected on
// (`TypeOps.asSeenFrom`): from a subclass, a private alias of the base class is no member
// (`MissingType`), in any clause, through `this`, an import or a prefix; `new Cmd(..)`, through
// every clause, and the class's own body see it.
object Scopes:
  trait Scope:
    private type Event = String
    case class Cmd(events: List[Event])
    def inside = Cmd(List("in"))
  val viaNew = new Scope:
    val made = new Cmd(List("1"))
  new Scope:
    val commands = List(Cmd(List("1", "2")))

object Clauses:
  trait Scope:
    private type Event = String
    case class Cmd(n: Int)(val events: List[Event])
    def inside = Cmd(0)(List("in"))
  object Derived extends Scope:
    val made = new Cmd(1)(List("a", "b"))
    val cmd = Cmd(1)(List("x"))
  class D extends Scope
  def prefixed(d: D) = d.Cmd(2)(List("y"))
