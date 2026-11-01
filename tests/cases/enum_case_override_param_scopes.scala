// teq: --werror
// An enum case's `override val` parameter where the enum extends a class, passes its own parameter on as the
// class's (`SuperParamAlias`), is nested in an object or a block, or qualifies its parameter `private[p]` for a scope
// the case is inside (`isAccessibleFrom` from the case): no diagnostic.
package p {
  enum S(private[p] val s: Int):
    case T(override val s: Int) extends S(s)
}

class A(val s: Int)

enum ClassParent(override val s: Int) extends A(s):
  case P(override val s: Int) extends ClassParent(s)

enum Through(s: Int) extends A(s):
  case Q(override val s: Int) extends Through(s)

object O:
  enum N(val s: Int):
    case M(override val s: Int) extends N(s)

class Outer:
  def f(): Int =
    enum L(private[Outer] val s: Int):
      case R(override val s: Int) extends L(s)
    L.R(5).s

@main def run(): Unit =
  enum Local(val s: Int):
    case V(override val s: Int) extends Local(s)
  println(p.S.T(1).s)
  println(ClassParent.P(2).s)
  println(Through.Q(3).s)
  println(O.N.M(4).s)
  println(Outer().f())
  println(Local.V(6).s)
