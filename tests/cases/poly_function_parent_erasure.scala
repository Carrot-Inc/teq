//> using platform jvm
// teq: --wconf msg=cannot.be.checked.at.runtime:s
// A refinement of `apply` is a polymorphic function type wherever its parent derives from `PolyFunction`
// (`PolyFunctionOf`'s `parent.derivesFrom(defn.PolyFunctionClass)`), an intersection's side or a type parameter's
// bound included, so each erases to the function class of its arity, in a descriptor as in a type test. A union with
// a type parameter bounded by `Null` derives from `PolyFunction` too (`isLowerBottomType` through the bound).
trait S
object P:
  type PS = (PolyFunction & S) { def apply(x: Int): Int }
  def f(p: PS): String = "f"
  def u(p: PS | Null): String = "u"
  type PN[N <: Null] = (PolyFunction | N) { def apply(x: Int): Int }
  def n[N <: Null](p: PN[N]): String = "n"
  def main(args: Array[String]): Unit =
    val function = classOf[Function1[?, ?]]
    println(P.getClass.getMethod("f", function).invoke(P, null))
    println(P.getClass.getMethod("u", function).invoke(P, null))
    println(P.getClass.getMethod("n", function).invoke(P, null))
    val any: Any = (x: Int) => x + 1
    println(any.isInstanceOf[PS])
