// `Any`'s `toString()` and `hashCode()` stay alternatives beside a member of the name that does not
// match them (`Denotation.matches`, `computeMembersNamed`): `()` applies `Any`'s where the class's
// own takes no list or needs arguments (`resolveOverloaded`), and the own one answers its own call.
import scala.annotation.targetName

class G(n: Int):
  @targetName("otherString") def toString[A]: String = s"other $n"

class K:
  @targetName("otherHash") def hashCode[A]: Int = -7

class D0:
  def toString(x: Int): String = "d" + x

@main def run(): Unit =
  println(G(3).toString().take(2))
  println(G(3).toString[Int])
  val k = K()
  println(k.hashCode() == System.identityHashCode(k))
  println(k.hashCode[Int])
  println(D0().toString().take(3))
  println(D0().toString(1))
