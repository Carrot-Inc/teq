package dma

// The parts of the intersections the downstream selects on, read from the products.
trait G[A] { def get: A }
class GS extends G[String] { def get: String = "same" }

trait Over { def g(x: Int): Any = "over-int"; def g(x: Boolean): Any = "over-bool" }
trait Under { def g(x: Int): String; def g(x: String): String }
class OverUnder extends Over with Under {
  override def g(x: Int): String = "met-int"
  def g(x: String): String = "under-string"
}

trait PA { def n: Int }
trait PB { def n: Int }
class PAB extends PA with PB { def n = 7 }
class PBA extends PB with PA { def n = 8 }
