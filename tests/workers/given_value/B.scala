import scala.quoted.*
inline def greet: String = ${ greetImpl }
def greetImpl(using Quotes): Expr[String] = Expr(summon[Greeter].hi)
