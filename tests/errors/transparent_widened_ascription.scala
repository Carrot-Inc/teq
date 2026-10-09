// expect: 6:22: error: type mismatch: found Any, required Int
// A transparent inline method whose body an ascription widens gives the widened type: its
// constant does not make the expansion an Int (scalac: E007 "Found: Any, Required: Int").
transparent inline def widened: Any = (1: Any)
def onlyInt(x: Int): Int = x
val result = onlyInt(widened)
@main def main(): Unit = println(result)
