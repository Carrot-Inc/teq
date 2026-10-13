// A qualified private member a trait declares is inherited (dotty drops only a plain `private` one) and
// is in scope in the body of an object extending the trait as in a class's; master did not find it there.
package p
trait T { private[p] def f: Int = 7 }
class C extends T { val x = f }
object O extends T { val y = this.f; val x = f }
@main def run(): Unit = println((new C).x + O.x + O.y + O.f)
