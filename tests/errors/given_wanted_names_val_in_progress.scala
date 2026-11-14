// expect: 7:7: error: recursive value alias needs type
// expect: 1 error found
// The wanted type names the val whose initialiser asks for the given: the type written there reads
// the val, which scalac reports as E045 "Recursive value alias needs type"; the search that follows
// adds no error of its own.
object Stock:
  val alias = { val o = summon[Ordering[Stock.alias.type]]; 3 }

@main def run(): Unit = println(Stock.alias)
