package privatealiasmisuse

// A case class nested in a trait whose parameters name the trait's private alias: the products
// state the written `Scope.this.Event` (the constructor's, its companion `apply`'s), which the
// trait's own members and a subclass's `new` read as the alias.
trait Scope:
  private type Event = String
  case class Cmd(n: Int)(val events: List[Event])
  def make: Cmd = Cmd(1)(List("in"))
  def size(c: Cmd): Int = c.events.size

object Derived extends Scope:
  val made = new Cmd(2)(List("a", "b"))
