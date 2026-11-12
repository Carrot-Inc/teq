// Applications.resolveOverloaded selects the alternative of each call by its own arguments
// (narrowByTypes, isApplicableMethodRef) and the typed Apply keeps that denotation: two calls of one
// name with different argument types keep two different methods, whichever is typed first.
@main def run(): Unit =
  val s = "banana"
  println(selected(s.replace('a', 'o')))
  println(selected(s.replace("an", "AN")))
  println(selected(s.replace('n', 'm')))
  println(s.replace('a', 'o'))
  println(s.replace("an", "AN"))
