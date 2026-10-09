// The scalac side of jar_dependent_functions.scala: compiled into a jar by scalac 3.8.4, never by
// teq. Function types whose result names a parameter, an ordinary and a contextual one, in the
// signatures of vals and defs.
package depfunlib

trait Ctx:
  type T
  def make: T
  def show(t: T): String

object IntCtx extends Ctx:
  type T = Int
  def make: Int = 7
  def show(t: Int): String = s"int $t"

object StrCtx extends Ctx:
  type T = String
  def make: String = "s"
  def show(t: String): String = s"str $t"

object Fns:
  val plain: (c: Ctx) => c.T = c => c.make
  val contextual: (c: Ctx) ?=> List[c.T] => Int = xs => xs.size
  val showing: (c: Ctx) => c.T => String = c => t => c.show(t)
  def run(fn: (c: Ctx) => c.T): Any = fn(IntCtx)
  def withCtx(fn: (c: Ctx) ?=> List[c.T] => Int): Int = fn(using IntCtx)(List(1, 2, 3))
  def made(using c: Ctx): c.T = c.make
