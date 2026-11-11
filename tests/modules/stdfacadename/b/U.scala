// A downstream of a/, built with a classpath (a/'s products), where the whole build has none. Typing `name + a`
// misses a member of String (`String.apply`, overloading resolution's shape test), which with a classpath enters
// String's Java members and the std's implementations of them (`std_string_implementations`), every std file defining
// a top-level `scala.<name>` of a String method, the Scala.js DOM files among them: `std/scalajs/dom_nodes.scala`'s
// `@JSGlobal("Node") class Node` among the classes. The emitter kept every facade's global off the output's names,
// reached or not, so the bundle over the products named a/'s `Node` `Node$$` where the whole build's and master
// efddb38de3's name it `Node` (tests/modules' bundle row, seqpattern's alike, a regression of the type-representation
// push); a facade's global is kept off where the output reaches the facade alone (tests/cases/js_global_facade_name).
package nb

import na.*

@main def run(): Unit =
  val n = new Node("n", 1, 2)
  n match
    case Node(name, a, rest*) => println(name + a + rest.sum)
