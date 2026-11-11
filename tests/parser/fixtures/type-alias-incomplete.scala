// A missing alias type before the next definition. teq's recovery after a syntax error is its own: the first
// error is scalac's (the same place, the same sentence), `def f` is kept, and no error follows. scalac's skip
// consumes `def f`, so its `println(f)` then reports f not found (the departure listed).
type T =
def f: Int = 1
@main def run(): Unit = println(f)
