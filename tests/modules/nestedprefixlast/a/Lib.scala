package npl

// The last fold's consumers across products: a trait of an abstract val (a pure interface: its
// type pattern takes no outer test, and a class that mixes it in defines no outer accessor for
// it), a secondary constructor of an inner superclass (`extends H.o.I()`, its outer passed and
// its `this(3)` pickled), and a bare extractor inside its class (`case J(k)` is `O.this.J`).
class O(val n: Int):
  trait V:
    val v: Int
  class I(val x: Int):
    def this() = this(3)
    def value = n + x
  case class J(k: Int)
  def test(x: O#J): String = x match
    case J(k) => "same"
    case _ => "other"

object H:
  val o = new O(7)
