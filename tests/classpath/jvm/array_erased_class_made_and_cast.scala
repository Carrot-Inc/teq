// jars: scala-library
// The array an element type makes, and the class a cast to the array type asks, are the class scalac's erasure
// gives the element: a value class with `Null` is an `Object` (made and cast so; the type test and the
// signatures already are), a `*:` chain spelled with `EmptyTuple` a `Product`. An array of such arrays then takes
// an `Object[]` made elsewhere (a library's) as a member.
class V(val x: Int) extends AnyVal
trait Tr extends Any
def cast(x: Any): String = try "len" + x.asInstanceOf[Array[V | Null]].length catch { case e: ClassCastException => "CCE" }
def castTr(x: Any): String = try "len" + x.asInstanceOf[Array[Tr | Null]].length catch { case e: ClassCastException => "CCE" }
@main def run(): Unit =
  def cls(x: Any): String = x.getClass.getName
  println("1 " + cls(Array[V | Null](new V(1), null)) + " " + cls(new Array[V | Null](1)))
  println("2 " + cls(new Array[Int *: EmptyTuple](1)) + " " + cls(Array[Int *: String *: EmptyTuple]((1, "a"))))
  val made: Array[V | Null] = new Array[V | Null](1)
  println("3 " + cls(Array(made, made)))
  val strs: Any = Array("a"); val ints: Any = Array(1)
  println("4 " + cast(strs) + " " + cast(ints) + " " + cast(made) + " " + castTr(strs) + " " + castTr(ints))
  val elsewhere: Array[V | Null] = new Array[AnyRef | Null](1).asInstanceOf[Array[V | Null]]
  println("5 " + (try Array(elsewhere, made).length.toString catch { case e: ArrayStoreException => "ArrayStoreException" }))
