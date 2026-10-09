// `x.asInstanceOf[T]` tests the value against `T`'s erasure, as scalac's `checkcast` does: a
// class, a trait, a function's arity, `Null` fail with `ClassCastException`, whether the result
// is discarded, used, cast again, widened to `Any` or compared in a guard; `null` passes; a cast
// to `Nothing` fails whatever the value. The message is the JVM's up to its module suffix.
class A
class B
trait T
class C extends T

def attempt(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case e: ClassCastException => println(name + ": " + e.getMessage.takeWhile(_ != '(').trim)

var effects = 0

@main def run(): Unit =
  val a: Any = new A
  attempt("class, discarded") { a.asInstanceOf[B]; () }
  attempt("class, used") { val b: B = a.asInstanceOf[B]; b == null }
  attempt("chained") { val x: A = a.asInstanceOf[B].asInstanceOf[A]; x }
  attempt("widened") { val x: Any = a.asInstanceOf[B]; effects += 1; x }
  println(effects)
  attempt("guard") {
    a match
      case x if x.asInstanceOf[B] != null => println("guard taken")
      case _ => println("other")
  }
  attempt("trait") { a.asInstanceOf[T]; () }
  println((new C: Any).asInstanceOf[T].isInstanceOf[T])
  attempt("boxed Int to String") { 7.asInstanceOf[String]; () }
  attempt("null to B") { val n: Any = null; println(n.asInstanceOf[B] == null) }
  attempt("Nothing of null") { null.asInstanceOf[Nothing] }
  attempt("Nothing") { a.asInstanceOf[Nothing] }
  attempt("Null") { a.asInstanceOf[Null]; () }
  attempt("Null of null") { val n: Any = null; n.asInstanceOf[Null] }
  println(("x": Any).asInstanceOf[Unit] == ())
  val f: Any = () => 1
  try
    f.asInstanceOf[Int => Int]
    println("arity passed")
  catch case _: ClassCastException => println("arity CCE")
  attempt("Function0") { f.asInstanceOf[() => Int]; () }
