// An inherited `private[p]` def of a trait is found from an object in `p`: the design pass's witness of TASKS 307,
// closed on master.
package p
trait T { private[p] def f(): Int = 7 }
class C extends T
object O { def test(): Int = new C().f() }
@main def run(): Unit = println(O.test())
