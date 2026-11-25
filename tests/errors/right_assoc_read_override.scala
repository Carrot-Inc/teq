// A right-associative extension cannot override an ordinary method, though both take `(r: String)(l: Int)`.
trait Ordinary:
  def +:(r: String)(l: Int): Int
class Ext extends Ordinary:
  extension (l: Int)
    override def +:(r: String): Int = l
@main def run(): Unit = ()
// expect: 6:18: error: error overriding method +: in trait Ordinary of type (r: String)(l: Int): Int;
