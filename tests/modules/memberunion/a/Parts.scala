package mua

// The parts of the intersections the downstream selects on, read from the products: one class
// whose instances the downstream intersects, and two traits whose members meet or stay apart.
trait R[K <: String] {
  def get(k: K): Any
  def poly[T](k: K, x: T): T
  def same(k: K): K
}
class Store(tag: String) extends R[String] {
  def get(k: String): Any = tag + ":" + k
  def poly[T](k: String, x: T): T = x
  def same(k: String): String = k + k
}

trait A { def f(x: String): Product }
trait B { def f(x: String): Serializable }
class AB extends A with B { def f(x: String): Product & Serializable = Some(x) }

trait S[K <: String] extends R[K] { override def get(k: K): String }
class SS extends Store("ss") with S[String] { override def get(k: String): String = "got " + k }
