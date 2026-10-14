package itp

// A top-level inline method whose body calls an inline method on a top-level val: the val is a
// stable path of the file's package object, which an expansion names without binding it, read
// whole as `Static` or through the products as the object's member (`Field` of its module).
class Lib:
  inline def twice(inline x: Int): Int = x + x
val lib = new Lib
inline def g(inline x: Int): Int = lib.twice(x)
