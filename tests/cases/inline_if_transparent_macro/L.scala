import scala.quoted.*
object L:
  transparent inline def threshold: Int = ${ thresholdImpl }
  def thresholdImpl(using Quotes): Expr[Int] = Expr(2)
  inline def log(inline lvl: Int, inline msg: String): Unit =
    inline if lvl >= threshold then println(msg)
