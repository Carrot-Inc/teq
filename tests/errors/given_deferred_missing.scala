// The implementation of a deferred given is searched where its class is defined (dotty's
// `Typer.implementDeferredGivens`): a given at the construction site, one of the class's body,
// one it inherits or one of a subclass's using clause does not serve, and an abstract class is
// the first to implement it as any class is.
// expect: 13:7: error: No given instance of type Int was found for inferring the implementation of the deferred given instance x in trait T
// expect: 14:7: error: No given instance of type Int was found for inferring the implementation of the deferred given instance x in trait T
// expect: 16:7: error: No given instance of type Int was found for inferring the implementation of the deferred given instance x in trait T
// expect: 17:16: error: No given instance of type Int was found for inferring the implementation of the deferred given instance x in trait T
import scala.compiletime.deferred

trait T { given x: Int = deferred }
class Base { given provided: Int = 11 }
class C extends T
class Body extends T { given supplied: Int = 11 }
@main def run(): Unit = { given Int = 22; println((new C).x) }
class Inherits extends Base with T
abstract class A extends T
class Below(using Int) extends A
