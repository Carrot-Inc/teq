import scala.quoted.*
object M:
  private var n = 0
  inline def arg: Int = ${ argImpl }
  transparent inline def tick: Unit = ${ tickImpl }
  def argImpl(using Quotes): Expr[Int] =
    n += 1
    if n == 1 then quotes.reflect.report.error("abandoned error")
    Expr(1)
  def tickImpl(using Quotes): Expr[Unit] =
    n += 1
    '{ () }
