// Earlier clauses participate in free-variable and mutable-cell analysis.
def combine(read: () => Int)(add: Int): Int = read() + add

@main def curriedCaptureClauses(): Unit =
  var current = 1
  val direct = (add: Int) => combine(() => current)(add)
  val partial: Int => Int = combine(() => current)
  current = 5
  println(direct(2))
  current = 6
  println(partial(2))
