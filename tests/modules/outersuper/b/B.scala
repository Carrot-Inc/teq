// A class of another module nested in a class is constructed under the descriptor its class file declares, its
// enclosing instance first (the path of the `extends` clause or of the `new`): a parent call, a creation, an
// anonymous class; the primary and an overloaded constructor; defaults, two clauses, named arguments. A creation
// whose class file holds two constructors that fit the primary's parameters (a secondary with the instance has the
// primary's own list) is the primary's.
import inc.*

def mark(s: String, n: Int): Int = { print(s); n }
object Path:
  lazy val api: Api = { print("p"); new Api(10) }

class Sub(k: Int) extends Stock.api.Request(k)
class Clauses(k: Int) extends Stock.api.Two(k)(2)
class Bare extends Stock.api.Defaults
class Named extends Path.api.Over(j = mark("j", 2), k = mark("k", 3))
class OverSub extends Stock.api.Over(3, 4)

@main def main(): Unit =
  println(new Sub(3).sum)
  println(new Clauses(3).sum)
  println(new Bare().sum)
  println(new Named().sum)
  println(new OverSub().sum)
  println(new Stock.api.Over(3, 4).sum)
  println(new Stock.api.Node(new Api(1), "leaf").path)
  println((new Stock.api.Request(5) { override def sum: Int = super.sum * 2 }).sum)
