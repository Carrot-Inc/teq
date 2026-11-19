// expect: 43:31: error: Found:    "s"⏎Required: Int
// expect: 45:35: error: Found:    "s"⏎Required: Int
// expect: 47:31: error: Found:    "s"⏎Required: Int
// expect: 49:28: error: Found:    "s"⏎Required: Int
// expect: 51:23: error: constructor P2 cannot be accessed as a member of P2 from class H
// expect: 53:20: error: constructor K cannot be accessed as a member of K from class KD
// expect: 54:21: error: constructor K cannot be accessed as a member of K from class KD
// expect: 55:30: error: constructor K cannot be accessed as a member of K from class KD
// expect: 56:17: error: the constructor of P3 is private to P3
// expect: 57:18: error: the constructor of P1 is private to P1
// expect: 59:11: error: constructor P2 cannot be accessed as a member of P2 from here
// expect: 60:11: error: constructor P2 cannot be accessed as a member of P2 from here
// expect: 61:11: error: the constructor of P3 is private to P3
// expect: 62:11: error: constructor P4 cannot be accessed as a member of P4 from here
// expect: 63:20: error: Found:    "s"⏎Required: Int
// expect: 64:12: error: the constructor of P3 is private to P3
// expect: 71:18: error: the constructor of S2 is private to S2
// expect: 73:11: error: the constructor of S2 is private to S2
// expect: 74:11: error: constructor S3 cannot be accessed as a member of S3 from here
// expect: 78:7: error: the constructor of TP is private to TP
// expect: 79:8: error: the constructor of TP is private to TP
// expect: 80:31: error: the constructor of TP is private to TP
// expect: 84:7: error: the constructor of P is private to P
// expect: 88:18: error: the constructor of PT is private to PT
// expect: 92:18: error: constructor G cannot be accessed as a member of G from class GD
// expect: 93:19: error: constructor G cannot be accessed as a member of G from object GO
// expect: 95:12: error: constructor G cannot be accessed as a member of G from here
// expect: 96:16: error: constructor G cannot be accessed as a member of G from here
// expect: 41:26: error: Found:    "s"⏎Required: Int
// expect: 29 errors found
package up:
  class P1 private (val x: Int)
  class P2 protected (val x: Int)
  class P3 private[up] (val x: Int)
  class P4 protected[up] (val x: Int)
  class S(val x: Int):
    protected def this(s: String) = this(1)
    private[up] def this(a: Int, b: Int) = this(2)
  case class K protected (x: Int)
  object InPkg:
    def bad(): S = new S("s")
class E extends up.S(0):
  def body(): up.S = new up.S("s")
  class In:
    def nested(): up.S = new up.S("s")
object E:
  def comp(): up.S = new up.S("s")
class F(val s: up.S)
class G extends F(new up.S("s"))
class H extends up.P2(1):
  def body(): up.P2 = new up.P2(2)
class KD extends up.K(1):
  def mk(): up.K = up.K(2)
  def mk2(): up.K = up.K.apply(3)
  def other(k: up.K): up.K = k.copy(x = 4)
class L extends up.P3(1)
class L1 extends up.P1(1)
@main def main(): Unit =
  println(new up.P2(1).x)
  println(up.P2(1).x)
  println(new up.P3(1).x)
  println(new up.P4(1).x)
  println(new up.S("s").x)
  println((new up.P3(1) {}).x)
// Of several constructors, all hidden, none is applied: one line a creation.
package hid:
  class S2 private (val x: Int):
    private def this(s: String) = this(1)
  class S3 protected (val x: Int):
    protected def this(s: String, t: String) = this(1)
class HD extends hid.S2("s")
def hidden(): Unit =
  println(new hid.S2("s").x)
  println(new hid.S3("s", "t").x)
// A superclass that comes through a trait is called by the class that extends the trait.
class TP private ()
trait TT extends TP
class TD extends TT
object TO extends TT
def through(): Unit = println(new TT {})
package tq:
  class P private[tq] (val x: Int = 7)
  trait T extends P
class QD extends tq.T
// A trait's private constructor is asked of a trait that extends it.
trait PT private ():
  def v = 1
trait PU extends PT
// A parent call that leaves its type arguments to inference is typed ahead of the super call.
package gen:
  class G[T] protected (val x: T)
class GD extends gen.G("s")
object GO extends gen.G(1)
def ahead(): Unit =
  println((new gen.G("s") {}).x)
  println((new gen.G("s") { def y = 1 }).x)
