package epsilon

// Compiled at -source:3.3, where a context bound's evidence takes an explicit argument in a plain clause: dotty's
// Migrations.contextBoundParams warns of it from 3.4 and refuses it from 3.5.
object Earlier:
  def f[T: Ordering](x: T): T = x
  val value = "earlier=" + f[Int](1)(Ordering.Int)
