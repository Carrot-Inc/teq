// expect: no given instance of type Name was found for parameter n
// expect: ambiguous given instances for Line: first, second
// jars: sourcecode
package demo

// Only the library's own classes have givens from its macros.
case class Name(value: String)

def nameOf(using n: Name): String = n.value

given first: sourcecode.Line = sourcecode.Line(1)
given second: sourcecode.Line = sourcecode.Line(2)

def lineOf(using l: sourcecode.Line): Int = l.value

@main def run(): Unit =
  println(nameOf)
  println(lineOf)
