// A case object takes `equals` from its superclass where the superclass has one, here from a trait of a class in
// another package: scalac makes no `equals` for a case object (`SyntheticMembers.caseModuleSymbols`). In the
// modules of the split output the object's registration can come before its superclass's, and the shape's own
// `equals`, reference equality, must not stay on the object then; one from a trait the object itself mixes in
// stays. An enum's values and a case class were right.
package zz:
  trait Loose:
    override def equals(that: Any): Boolean = that.isInstanceOf[Loose]
    override def hashCode: Int = 7
  abstract class Zed extends Loose
  trait Strict:
    override def equals(that: Any): Boolean = that.isInstanceOf[String]

package aa:
  abstract class Mid extends zz.Zed
  case object Through extends Mid
  case object Direct extends zz.Zed
  object Outer:
    case object Inner extends zz.Zed
  enum Color extends zz.Zed:
    case Red, Green
  enum Shape extends zz.Zed:
    case Dot
    case Box(side: Int)
  case object Mixed extends zz.Zed, zz.Strict
  case object Own extends zz.Zed:
    override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this
  class Other extends zz.Loose

  @main def run(): Unit =
    val other = new Other
    println(Through == other)
    println(Direct == other)
    println(Outer.Inner == other)
    println(Color.Red == other)
    println(Shape.Dot == other)
    println(Shape.Box(1) == other)
    println(Mixed == other)
    println(Mixed.equals("a string"))
    println(Own == other)
    def local(): Boolean =
      case object Local extends zz.Zed
      Local == other
    println(local())
    println(List(Through.hashCode, Direct.hashCode, Color.Red.hashCode, Shape.Dot.hashCode))
