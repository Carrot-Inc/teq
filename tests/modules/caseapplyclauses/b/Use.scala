import creatorCalls.Calls
@main def run(): Unit =
  println(Calls.defaulted.age)
  println(Calls.repeated.values.mkString(","))
  println(Calls.contextual.label)
  println(Calls.curried.second)
  println(Calls.lifted.age)
  println(Calls.delayed.total)
  println(Calls.values.elements.mkString(","))
  println(Calls.emptyFirst.n)
  println(Calls.nested.value)
