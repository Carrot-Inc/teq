//> using platform js
// A unit a macro of the run suspends is typed again on its own tree (Run.compileSuspendedUnits): a JavaScript
// class's constructor parameter keeps its written `@JSName`, installed from its declaration as Namer installs it,
// so PrepJSInterop finds it again (.expected from scalac 3.8.4 -scalajs).
import scala.quoted.*

object M:
  inline def one: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = '{ 1 }
