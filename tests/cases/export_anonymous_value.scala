// Export clauses in anonymous classes, through a top-level val, an enclosing class's val, a path of
// two vals and an object, and in a local class through its block's parameter: completed where the
// class is made, as dotty's class completer runs `Namer.processExports`, their forwarders implement
// the parent's abstract members.
trait Count:
  def count: Int
trait Pair:
  def first: Int
  def second(n: Int): Int
class Impl:
  def count = 7
  def first = 1
  def second(n: Int) = n * 2
object Maker:
  def make(n: Int): String = "made " + n
trait Make:
  def apply(n: Int): String
class Rounds:
  val rounds = new Impl
val impl = new Impl
val c: Count = new Count:
  export impl.count
class Env:
  val socket = new Rounds
  val pair: Pair = new:
    export socket.rounds.{first, second}
  val make: Make = new:
    export Maker.make as apply
@main def main() =
  println(local(new Impl))
  val e = new Env
  println((c.count, e.pair.first, e.pair.second(5), e.make(3)))
def local(x: Impl): Int =
  class L extends Count:
    export x.count
  new L().count + 1
