// A `return` in a method nested in an inline body leaves that method, which scalac 3.8.4 allows at
// the definition; one that would leave the inline method is refused
// (`tests/errors/inline_definition_errors`).
inline def f: Int = { def g(): Int = return 1; g() }
@main def run(): Unit = println(1)
