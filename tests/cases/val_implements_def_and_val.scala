// A val that implements a def of one parent and a val of another is read through either: on
// JavaScript the val is reached through a method, and so is every val it overrides or
// implements and every val overriding it, whichever parent the read goes through and whichever
// order the parents come in. A val that implements a val alone still reads as a val.
trait D:
  def value: Int
trait V:
  val value: Int
trait V3:
  val value: Int = 1
abstract class AV:
  val value: Int

class C extends D with V:
  val value = 42
class R extends V with D:
  val value = 43
class OnlyV extends V:
  val value = 7
class OnlyV2 extends OnlyV:
  override val value = 8
class C3 extends D with V3:
  override val value = 2
class V3b extends V3:
  override val value = 9
class CA extends AV with D:
  val value = 5
case class P(value: Int) extends D with V
// A val a class inherits from one trait for another's, and an object implementing a val, once
// another class's val makes the val a method.
trait Given:
  val value: Int = 3
class Inherits extends Given with V
trait Read:
  def tag: String = "read"
trait DR:
  def item: Read
trait VR:
  val item: Read
class CR extends DR with VR:
  val item: Read = new Read {}
class ObjectItem extends VR:
  object item extends Read

def readD(x: D): Int = x.value
def readV(x: V): Int = x.value
def readV3(x: V3): Int = x.value
def readAV(x: AV): Int = x.value

@main def run(): Unit =
  val c = new C
  println(List(c.value, readD(c), readV(c)))
  val r = new R
  println(List(readV(r), readD(r), r.value))
  println(List(readV(new OnlyV), readV(new OnlyV2), new OnlyV2().value))
  println(List(readD(new C3), readV3(new C3), readV3(new V3b), readV3(new V3 {}), new V3b().value))
  println(List(readD(new CA), readAV(new CA)))
  val p = P(11)
  println(List(readD(p), readV(p), p.value, p.copy(value = 12).value, p))
  println(List(readV(new Inherits), (new Inherits: Given).value, new Inherits().value))
  println(List((new CR: DR).item.tag, (new CR: VR).item.tag, (new ObjectItem: VR).item.tag, new ObjectItem().item.tag))
