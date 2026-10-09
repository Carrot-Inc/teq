// A primitive cast to a primitive it has no conversion to (`tpd.primitiveConversion` finding
// no `toInt` on a `Boolean`, a `Unit`, no `toBoolean` on an `Int`) throws a
// ClassCastException with no message, its operand never run; one it has a conversion to is
// that conversion. The casts stand in inline methods, whose expansions decide them with the
// arguments' types, so that the program compiles without the warning of a direct one
// (tests/errors/cast_primitive_conversion.scala).
var n = 0
def b: Boolean = { n += 1; true }
def i: Int = { n += 10; 4 }
def u: Unit = { n += 100; () }
def c: Char = { n += 1000; 'A' }
inline def toInt[A](inline a: A): Int = a.asInstanceOf[Int]
inline def toBoolean[A](inline a: A): Boolean = a.asInstanceOf[Boolean]
inline def toChar[A](inline a: A): Char = a.asInstanceOf[Char]
inline def toLong[A](inline a: A): Long = a.asInstanceOf[Long]
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case e: ClassCastException => println(label + " CCE " + (e.getMessage == null))
@main def run(): Unit =
  attempt("boolean-int") { toInt(b); () }; println(n)
  attempt("int-boolean") { toBoolean(i); () }; println(n)
  attempt("unit-int") { toInt(u); () }; println(n)
  attempt("char-boolean") { toBoolean(c); () }; println(n)
  attempt("used") { println(toInt(b) + 2) }; println(n)
  attempt("char-int") { println(toInt(c)) }; println(n)
  attempt("int-char") { println(toChar(i)) }; println(n)
  attempt("int-long") { println(toLong(i) + 1L) }; println(n)
