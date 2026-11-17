// jars: scala-library
// A typed pattern over an array of a universal trait with `Null` binds what its test takes. scalac 3.8.4 tests
// `Array[Tr | Null]` as an `Object[]` (its `isInstanceOf` and its cast do too) and casts the pattern's binder to
// `Tr[]`, so its own program dies at the case (ClassCastException) for an array of `String`, an array of `Any`
// and the array its own `Array[Tr | Null](..)` makes, an `Object[]`: the four lines marked. teq tests as scalac
// does and binds the value: the expectation is teq's lines, not scalac's.
trait Tr extends Any
class TBox extends Tr
def bind(x: Any): String = x match { case a: Array[Tr | Null] => "arr" + a.length; case _ => "no" }
def give(): Array[Tr | Null] = Array[Tr | Null](new TBox, null)
@main def run(): Unit =
  println("made " + bind(new Array[Tr | Null](1)))
  println("literal " + bind(Array[Tr | Null](new TBox, null))) // scalac: ClassCastException
  println("given " + bind(give())) // scalac: ClassCastException
  println("trs " + bind(Array[Tr](new TBox)))
  println("test " + (Array("a"): Any).isInstanceOf[Array[Tr | Null]])
  println("strs " + bind(Array("a"))) // scalac: ClassCastException
  println("objs " + bind(Array[Any](1))) // scalac: ClassCastException
  println("ints " + bind(Array(1)))
