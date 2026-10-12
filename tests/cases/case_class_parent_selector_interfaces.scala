//> using platform jvm
// split-watch: jvm
// A case class whose parent declares `_1` abstractly has it implemented by its selector, and its interfaces are the
// parent, then `Product, Serializable` in scalac's order (TASKS 22, 412; the design pass's witness, closed on master).
trait Parent { def _1: Int }
case class C(x: Int) extends Parent
@main def run(): Unit = { println(C(1)._1); println(C(1).getClass.getInterfaces.map(_.getName).mkString(",")); println(C(1).productIterator.mkString(",")) }
