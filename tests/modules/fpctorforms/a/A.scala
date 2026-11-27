package a2
class Va(xs: Int*) { def n: Int = xs.size + 31 }
trait VaT extends Va
class VaK extends VaT
class Dep(val b: Int = Dep.base + 11) { def n: Int = b }
object Dep { val base: Int = 20 }
trait DepT extends Dep
class DepK extends DepT
class Sec(val a: Int) { def this() = this(31); def n: Int = a }
trait SecT extends Sec
class SecK extends SecT
class ByN(b: => Int = 31) { def n: Int = b }
trait ByNT extends ByN
class ByNK extends ByNT
trait JT extends java.util.ArrayList[String]
class JK extends JT { def n: Int = size + 31 }
object U { def all: List[Int] = { class L1 extends VaT; class L3 extends DepT; class L4 extends SecT; class L5 extends ByNT; class L6 extends JT { def n: Int = size + 31 }
  List(new L1().n, new L3().n, new L4().n, new L5().n, new L6().n) } }
