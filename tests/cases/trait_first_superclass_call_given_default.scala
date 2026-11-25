// A class whose first written parent is a trait over a class calls the class's constructor (dotty's
// `Namer.ensureFirstIsClass`, `Typer.ensureConstrCall`): with no argument, with a synthesized given (`ValueOf`), with
// the given at a type argument, and with a default; from a class, an object, a local class of a method and an
// anonymous class, and from a subclass of each class.
class Base { def n: Int = 3 }
trait BT extends Base
class BK extends BT

class Mono(using val v: ValueOf[31]) { def n: Int = v.value }
trait MT extends Mono
class MK extends MT
object MO extends MT

class Generic[A](using val v: ValueOf[A]) { def n: A = v.value }
trait GT extends Generic[31]
class GK extends GT

class Plain(val k: Int = 7) { def n: Int = k }
trait PT extends Plain
class PK extends PT
object PO extends PT

object Main:
  def mono: Int =
    class L extends MT
    new L().n

  def plain: Int =
    class L extends PT
    new L().n

  def main(args: Array[String]): Unit =
    println(new BK().n)
    println(new MK().n)
    println(new GK().n)
    println(new PK().n)
    class Sub extends MK
    println(new Sub().n)
    class GSub extends GK
    println(new GSub().n)
    println((new PK {}).n)
    println((new MT {}).n)
    println(MO.n)
    println(PO.n)
    println(mono)
    println(plain)
