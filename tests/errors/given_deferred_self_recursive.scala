// A search that finds the deferred given itself through the implementing class, here a companion
// in the implicit scope of its type, is no implementation (dotty's `implementDeferredGivens`).
// expect: 8:8: error: Inferred implementation of the deferred given instance sh in trait T is self-recursive.
import scala.compiletime.deferred
trait Show[A] { def show(a: A): String }
trait T[A] { given sh: Show[A] = deferred }
case class Foo(i: Int)
object Foo extends T[Foo]

// A given of the object's own with parameters overloads the deferred one rather than implements it
// (dotty's tests/neg/i22589b): the search finds the deferred one through the object.
// expect: 16:8: error: Inferred implementation of the deferred given instance myc in trait Essentials is self-recursive.
trait MyCodec[E]
trait Essentials[E] { given myc: MyCodec[E] = deferred }
case class Person(name: String)
object Person extends Essentials[Person]:
  given String = "hw"
  given myc(using String): MyCodec[Person] = new MyCodec[Person] {}
