// A deferred given with parameters is not implemented by the search: the class implements it
// itself (dotty's `Typer.implementDeferredGivens`, the `Method` guard).
// expect: 10:7: error: Cannnot infer the implementation of the deferred given instance list in trait T
// expect: 11:7: error: Cannnot infer the implementation of the deferred given instance show in trait U
import scala.compiletime.deferred

trait T { given list[A]: List[A] = deferred }
trait U { given show(using Int): String = deferred }
given Int = 1
class C extends T
class D extends U
