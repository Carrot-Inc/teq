package returned

// A transparent method returning a function the middle module applies at once: the `INLINED`
// is the receiver of the selection of `apply`, which takes its own place where the receiver's
// source is not its own.
object A:
  transparent inline def fun(inline x: Int): Int => Int = y => x + y
