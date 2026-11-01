// A macro whose implementation runs the program's own classes at compile time: the expansion meets a macro of the run,
// so its file waits for a later round, and the classes' file is done with the phases after `Inlining` first
// (`Run.compileSuspendedUnits`), `Constructors.intoConstr` among them: B(2)'s A reads its parameter in its constructor
// code at compile time as at run time (scalac `3 3 3 3` twice).
import scala.quoted.*
abstract class Fn:
  def apply(): Int
class A(val s: Int):
  val saved = s
  val lam = () => s
  val sam: Fn = () => s
  inline def get = s
  val viaInline = get
class B(override val s: Int) extends A(s + 1)
object Mac:
  inline def atCompile: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    val b = B(2)
    Expr(s"${b.saved} ${b.lam()} ${b.sam()} ${b.viaInline}")
