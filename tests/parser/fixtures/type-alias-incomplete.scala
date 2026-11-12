// A missing alias type before the next definition. The expectation is the first
// error alone, which is scalac's (the same place, the same sentence), `def f` kept:
// scalac's skip consumes `def f`, so its `println(f)` then reports f not found.
type T =
def f: Int = 1
@main def run(): Unit = println(f)
