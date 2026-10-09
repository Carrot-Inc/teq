// A cast tests the erasure of its type (`TypeErasure.erasure`): a type argument is not tested,
// so `List[String]` of a `List[Int]` passes where an `Object` fails; a type parameter is its
// bound's erasure inside the generic body; an intersection is its erased glb, a union its
// erased lub, a literal type its class; `Tuple` is `Product` and `Singleton` is `Object`
// (`Definitions.specialErasure`).
class A
class B
trait T

def fails(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case _: ClassCastException => println(name + " CCE")

def unbounded[X](x: Any): Unit = { x.asInstanceOf[X]; println("unbounded passed") }
def bounded[X <: B](x: Any): Unit = { x.asInstanceOf[X]; println("bounded passed") }

@main def run(): Unit =
  val xs: Any = List(1)
  fails("List[String] of List[Int]") { xs.asInstanceOf[List[String]]; () }
  fails("List[String] of Object") { (new Object: Any).asInstanceOf[List[String]]; () }
  unbounded[B](new A)
  fails("bounded") { bounded[B](new A) }
  val x: Any = new A
  println(x.isInstanceOf[A & T])
  fails("intersection") { x.asInstanceOf[A & T]; () }
  fails("intersection of B") { x.asInstanceOf[B & T]; () }
  val y: Any = new Object
  println(y.isInstanceOf[A | B])
  fails("union") { y.asInstanceOf[A | B]; () }
  println(2.asInstanceOf[1])
  val pair: Any = (1, "a")
  fails("tuple") { pair.asInstanceOf[(String, Int)]; () }
  fails("tuple of A") { x.asInstanceOf[(String, Int)]; () }
  fails("Product of a case class") { (Some(1): Any).asInstanceOf[Product]; () }
  fails("Tuple of a case class") { (Some(1): Any).asInstanceOf[Tuple]; () }
  fails("Tuple of a pair") { pair.asInstanceOf[Tuple]; () }
  fails("Tuple of a String") { ("x": Any).asInstanceOf[Tuple]; () }
  fails("Singleton of a String") { ("x": Any).asInstanceOf[Singleton]; () }
