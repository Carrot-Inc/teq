// expect: 6:1: error: expected an identifier, found 'def'
// expect: 1 error found
// An alias with no type before the next definition is reported there, and the definition is
// kept, where scalac swallows it ("Not found: f"), as the parser's recovery has it.
type T =
def f: Int = 1
@main def run(): Unit = println(f)
