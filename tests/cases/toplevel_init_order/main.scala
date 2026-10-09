// A file's initialiser runs before the arguments of a call of its def, before the value assigned
// to its var and before a default argument, as scalac evaluates the `<file>$package` object
// first; a throwing argument still finds the file initialised.
@main def main(): Unit =
  println(oa.f({ println("argument"); 2 }))
  try println(ob.f({ println("throwing argument"); throw new RuntimeException("boom") }))
  catch case e: RuntimeException => println("caught " + e.getMessage)
  oc.v = { println("assigned value"); 9 }
  println(oc.v)
  println(od.withDefault())
  locally:
    import oe.*
    println({ println("receiver"); 1 }.plus({ println("extension argument"); 2 }))
