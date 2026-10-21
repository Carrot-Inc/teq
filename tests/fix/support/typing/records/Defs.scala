package records
object Defs {
  def choose(x: Int): Int = x
  def choose(x: String): String = x
  inline def ident[A](a: A): A = a
}
