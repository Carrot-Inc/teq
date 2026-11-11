package m
import scala.quoted.*

// The macro's run reaches `Vector.empty` of the standard library, whose body calls its object's own `wrap`.
object M:
  inline def n: Int = ${ nImpl }
  def nImpl(using Quotes): Expr[Int] = Expr(Vector.empty[Int].size + 1)
