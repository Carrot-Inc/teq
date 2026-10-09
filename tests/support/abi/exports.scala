// Item 3 of the JVM ABI alignment: an `export` makes scalac's
// forwarders, public final methods calling the member on the qualifier (or the qualifier's own forwarder of it),
// with the forwarders of its defaults' getters and the bridge an inherited member of the name needs, an exported
// object's and a case class's companion's answering the module, a trait's as default methods with their static
// `m$`; a class below one whose export implements an abstract member inherits it.
// An inherited export implementing a generic member gets its erased bridge in the class (`ChildG`); a forwarder keeps the member's `@targetName`, renamed or not, its getters' forwarders the forwarder's source name.
// abi: Car#start Car#start$default$1 Child#start Child#start$default$1 Utils$#namedEnc Utils#namedEnc Kit$#hammer Kit$#Inner Kit$#Nail Kit#hammer Kit#Inner Kit#Nail B$#f B$#f$default$1 B#f B#f$default$1 Relay$#renamed Relay#renamed Prelude#twice Prelude#twice$ Prelude#lucky Prelude#lucky$ ChildG#f(Ljava/lang/Object; Tn$#renamed Tn$#f$default$1 TnR$#renamed TnR$#g$default$1 Tn#renamed TnR#renamed
package exports
trait Base[A] { def start(x: A): A }
object Engine { def start(x: Int = 1): Int = x }
class Car extends Base[Int] { export Engine.start }
class Child extends Car
trait Named[A]
object Source:
  given namedEnc[A](using n: Named[A]): Ordering[A] = new Ordering[A] { def compare(x: A, y: A): Int = 0 }
object Utils:
  export Source.given
object Tools:
  def hammer: String = "hammer"
  object Inner { def x = 1 }
  case class Nail(length: Int)
object Kit:
  export Tools.{hammer, Inner, Nail}
object Over:
  def f(s: String): String = s
  def f(n: Int = 7): Int = n
object B:
  export Over.f
object Relay:
  export Over.{f as renamed}
object Lib:
  def twice(x: Int): Int = x * 2
  given lucky: Int = 7
trait Prelude:
  export Lib.{twice, given}
object ImplG { def f(x: String): String = x }
trait ParentG { export ImplG.f }
trait APIG[A] { def f(x: A): A }
class ChildG extends ParentG with APIG[String]
object TnO:
  @scala.annotation.targetName("renamed") def f(x: Int = 2): Int = x
object Tn:
  export TnO.f
object TnR:
  export TnO.{f as g}
