// As nested_trait_inferred_prefix.scala, the member that names the second val an abstract var:
// its setter's parameter has the var's type.
class Rack:
  trait Sized:
    def size: Int = 1
  trait Marked:
    var mark: Stock.second.Sized

object Stock:
  val first =
    val r = new Rack
    List(1, 2, 3).map(_ + 1)
    r
  val second =
    val r = new Rack
    List(1, 2, 3).map(_ + 1)
    r

class Crate extends Stock.first.Sized
abstract class Pallet extends Stock.first.Marked

object Use:
  val crate = new Stock.second.Sized {}

@main def run(): Unit =
  println(new Crate().size + Use.crate.size)
