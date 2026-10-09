// expect: 19:22: error: no given instance of type Missing was found for parameter
// expect: 23:20: error: type mismatch: found Boolean, required Float
// expect: 27:20: error: missing parameter type
// absent: required String
// The selection `q.op(a)` of a right-associative extension takes the extension once its prefix
// `op(q)` takes the qualifier (dotty's `tryExtension`): what fails after it is its error, not the
// conversion's turn (a String's `+:` would take each of these): a trailing given missing (E172), the
// argument of another type than the receiver (E007), a lambda against a receiver no function (E081).
trait Missing
object A:
  extension (n: Int) def +:(s: String)(using Missing): String = "ext"
object B:
  extension (n: Float) def +:(s: String): String = s
object C:
  extension (s: String) def +:(c: String): String = s
@main def run(): Unit =
  locally {
    import A.*
    println("x".+:(2))
  }
  locally {
    import B.*
    println("x".+:(true))
  }
  locally {
    import C.*
    println("x".+:(x => x))
  }
