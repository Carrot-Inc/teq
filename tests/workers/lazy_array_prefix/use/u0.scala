import scala.quoted.*

// A file with quoted code, typed before the fork: its expansion runs there, on the first
// worker's heap.
object U0:
  val x: Int = ma
  def quoted(using Quotes): Expr[Int] = '{ 1 }
