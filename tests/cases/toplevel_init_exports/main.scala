// The member an export clause of an object makes forwards to a top-level def, whose file it
// initialises after the arguments, as scalac's forwarder calls the def through the file's
// `<file>$package` object.
trait F { def f(x: Int): Int }
object O extends F { export oa.f }

@main def main(): Unit =
  val o: F = O
  println(o.f({ println("argument"); 2 }))
  println(O.f({ println("again"); 3 }))
