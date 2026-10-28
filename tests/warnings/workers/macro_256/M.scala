import scala.quoted.*
object M {
  inline def warn(inline msg: String): Unit = ${impl('msg)}
  def impl(msg: Expr[String])(using Quotes): Expr[Unit] = {
    import quotes.reflect.*
    report.warning(msg.valueOrAbort, '{Target.anchor}.asTerm.pos)
    '{ () }
  }
}
object Target { val anchor: Int = 0 }
