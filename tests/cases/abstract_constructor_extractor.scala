// A constructor or extractor pattern over an abstract type constructor's application (`M[T]`)
// is a class test of the scrutinee, its binder typed by the extractor's maximized argument
// (dotty's `typedUnApply` case 2, `maximizeType`): scala/pos/t4911.
class Foo[T](val x: T)
object Foo { def unapply[T](x: Foo[T]) = Some(x.x) }
case class Bar[T](x: T)

def f2[M[_], T](x: M[T]): Any = x match { case Foo(y) => y; case _ => "other" }
def f4[M[_], T](x: M[T]): Any = x match { case Bar(y) => y; case _ => "other" }

@main def run(): Unit =
  println(f2(new Foo(1)))
  println(f2(List(2)))
  println(f4(Bar("b")))
  println(f4(Option(3)))
