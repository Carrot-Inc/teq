// Classes extend traits nested in a class through a val of inferred type, and one trait's abstract
// method names the other trait through a second such val: the method's signature waits for a val
// another worker is typing.
class Rack:
  trait Sized:
    def size: Int = 1
  trait Fits:
    def fit(x: Stock.second.Sized): Int

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
abstract class Pallet extends Stock.first.Fits

object Use:
  val crate = new Stock.second.Sized {}

@main def run(): Unit =
  println(new Crate().size + Use.crate.size)
