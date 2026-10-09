import scala.quoted.*
object M:
  inline def runIt: Unit = ${ '{
    def force(x: => Unit): Unit = x
    force(println("forced"))
    println("done")
  } }
  inline def twice: Int = ${ '{ def force(x: => Int): Int = x + x; force(3) } }
  inline def show(expr: => Boolean): Unit = ${ showImpl('expr) }
  def showImpl(expr: Expr[Boolean])(using Quotes): Expr[Unit] = '{ println($expr) }
