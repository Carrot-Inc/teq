package eiv

import scala.compiletime.error

// Inline members exported through a top-level value and through a class's value: each forwarder
// inline, its body the member's call on the path, pickled so that a downstream expands it.
class Lib:
  transparent inline def f(inline x: Int): Int = inline if x == 1 then 7 else error("bad")
  inline def twice(inline x: Int): Int = x + x
  transparent inline def pick(inline b: Boolean): Any = inline if b then "text" else 42

val lib = new Lib
export lib.{f, pick}

class Proxy(val lib: Lib):
  export lib.{f, twice}
