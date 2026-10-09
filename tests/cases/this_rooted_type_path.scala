// A type path may start at `this`, at `C.this` or at a self alias: `this.b.N`, `t.b.N`.
trait U:
  type N
  def n: N

trait T:
  t =>
  val b: U
  def show: String =
    val v: this.b.N = b.n
    val w: t.b.N = v
    val z: T.this.b.N = w
    val same: List[t.b.N] = List(v, w, z)
    same.mkString(",")

object O extends T:
  val b = new U:
    type N = Int
    def n = 42

@main def Main(): Unit =
  println(O.show)
