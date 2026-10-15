// An `implicit` lambda without a parameter type where a context function is expected: dotty takes a
// closure's parameter types from the expected function type only for a closure that is contextual
// itself (`Typer.typedFunctionValue` tests `Given`, `untpd.isContextualClosure`), and wraps any other
// expression in a context closure of its own (`Typer.typedUnadapted`, `makeContextualFunction`), whose
// body, the written lambda, is then expected to be the result type: nothing gives its parameter a
// type, E081, an inline parameter's argument the same (typed against `formal.widenExpr` first,
// `ProtoTypes`; the inline context is the callee's body, not the call). scalac 3.8.4 reports each.
// expect: 17:32: error: missing parameter type
// expect: 18:26: error: missing parameter type
// expect: 19:29: error: missing parameter type
// expect: 20:31: error: missing parameter type
class U
object Test:
  inline def useInline[A](inline f: U ?=> A): A = f(using new U)
  def use[A](f: U ?=> A): A = f(using new U)
  def useInt(f: U ?=> Int): Int = f(using new U)
  val a = useInline { implicit u => summon[U]; 1 }
  val b = use { implicit u => summon[U]; 1 }
  val c = useInt { implicit u => summon[U]; 1 }
  val d = use[Int] { implicit u => summon[U]; 1 }
