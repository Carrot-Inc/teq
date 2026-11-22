// jars: scala-library givenfold-lib
// A library's inline method that calls another without its default argument, expanded here: the client's used names
// are scalac's, the default getter `walk$default$1` and the operator `$plus` of the body's `idx + 1` among them.
package lvn

import givenfold.Walk

object Use:
  def size: Int = Walk.size[(Int, String, Boolean)]
