// teq: --werror
// A val parameter overridden through a chain of parameter accessors (RefChecks (1.12)): a private one passed on
// unchanged to its superclass's like-named parameter (`SuperParamAlias`: written, inferred from a using list,
// ascribed), the ancestor's accessible from the class checked (inside its `private[p]` package or class, its
// companion, protected). One passed another value is tests/cases/override_param_values.
class A(val s: Int)
class C(s: Int) extends A(s)
class D(override val s: Int) extends C(s)

package p {
  class PA(private[p] val s: Int)
  class PB(override private[p] val s: Int) extends PA(s)
  class PC(override val s: Int) extends PB(s)
}

class Outer:
  class OA(private[Outer] val s: Int)
  def local(): Int =
    class LA(private[Outer] val s: Int)
    class LC(override val s: Int) extends LA(s)
    LC(7).s
object Outer:
  val o = new Outer
  class OC(override val s: Int) extends o.OA(s)

class Prot(protected val s: Int)
class ProtC(override val s: Int) extends Prot(s)

class UA()(using val s: Int)
class UB()(using s: Int) extends UA()
class UC()(using override val s: Int) extends UB()

class TypedA(val s: Any)
class TypedB(s: Int) extends TypedA(s: Int)
class TypedC(override val s: Int) extends TypedB(s)

@main def run(): Unit =
  println(D(2).s)
  println(p.PC(4).s)
  println(Outer.OC(5).s)
  println(Outer().local())
  println(ProtC(8).s)
  println(UC()(using 10).s)
  println(TypedC(11).s)
