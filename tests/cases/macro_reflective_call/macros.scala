// A macro calls members reflectively, as a library that supports several compiler versions does:
// `getClass.getMethod(name).invoke(receiver)` on `CompilationInfo`, whose type is scalac's
// `CompilationInfoModule`, and on an instance of a class of the program.
import scala.quoted.*

class Greeter(msg: String):
  def greet: String = msg + "!"
  def twice(n: Int): Int = n * 2

object Macros:
  inline def report: String = ${ reportImpl }

  def reportImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val info: CompilationInfoModule = CompilationInfo
    val settings = info.getClass.getMethod("XmacroSettings").invoke(info).asInstanceOf[List[String]]
    val g = new Greeter("hi")
    val greeting = g.getClass.getMethod("greet").invoke(g)
    val missing =
      try g.getClass.getMethod("wave").getName
      catch case _: NoSuchMethodException => "no wave"
    Expr(s"settings=${settings.size} $greeting ${g.getClass.getMethod("greet").getName} $missing")
