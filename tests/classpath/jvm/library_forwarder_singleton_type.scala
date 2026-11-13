// jars: rounddemand-lib
// A library's export forwarders of a val and of an object, named in singleton types alone (a type argument,
// an alias, a cast): the forwarders are stable methods, paths as their members are, whatever of the library
// has been read when the type is met. Only the cast's read initialises the exporting object.
package paths

import rounddemand.Counter

type Plain = Counter.plain.type

def size(xs: List[Counter.plain.type]): Int = xs.size
def length(x: Plain): Int = x.length
def count(xs: List[Counter.Inner.type]): Int = xs.size

@main def run(): Unit =
  println(size(Nil))
  println(length(Counter.plain.asInstanceOf[Plain]))
  println(count(Nil))
