// An inline match binds its scrutinee to a local, which the reduced case's body reads past to
// the definition named as it is.
def `scrutinee$0`(): Int = 7
def name(): String = "abc"

inline def size(inline x: Any): Int = inline x match
  case s: String => s.length + `scrutinee$0`()
  case _ => 0

@main def main(): Unit =
  println(size(name()))
