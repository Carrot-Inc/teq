// A value class's `equals` and `hashCode` are the rule's where the only member of the name that a base holds is no
// matching one: an `equals` that goes by another target name (scalac 3.8.4's `existingDef` asks for a member of the
// rule's name, parameter and target name), or an overload of another parameter.
import scala.annotation.targetName

trait Loose extends Any { @targetName("equalsRef") def equals(o: AnyRef): Boolean = false }
class Below(val x: Int) extends AnyVal with Loose

trait Overloads extends Any {
  def equals(o: Overloads): Boolean = false
  def hashCode(seed: Int): Int = seed
}
class Beside(val x: Int) extends AnyVal with Overloads

trait Always extends Any {
  override def equals(o: Any): Boolean = true
  override def hashCode: Int = 7
}
class First(val x: Int) extends AnyVal with Always with Overloads
class Second(val x: Int) extends AnyVal with Overloads with Always

@main def run(): Unit =
  def show(label: String, a: Any, b: Any, c: Any): Unit =
    println(s"$label ${a == b} ${a == c} ${a.equals(b)} ${a.hashCode == b.hashCode} ${a.hashCode}")
  show("Below", new Below(1), new Below(1), new Below(2))
  println(Set(new Below(1), new Below(1)).size)
  println(List(new Below(1)).contains(new Below(1)))
  show("Beside", new Beside(1), new Beside(1), new Beside(2))
  println(Set(new Beside(1), new Beside(1)).size)
  println(new Beside(1).hashCode(3))
  show("First", new First(1), new First(1), new First(2))
  show("Second", new Second(1), new Second(1), new Second(2))
  println(Set(new First(1), new First(2)).size)
