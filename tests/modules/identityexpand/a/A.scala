package idx

// A transparent method whose expansion is its argument: the middle module's pickle places the
// root the expansion took from its call site at the call site's place in the whole build as in
// the split one.
object A:
  transparent inline def id(inline x: Int): Int = x
