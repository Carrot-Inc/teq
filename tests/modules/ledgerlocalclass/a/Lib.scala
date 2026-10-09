package lla

// The inline walk, through a product: two expansions of a body that makes a
// named local class give a class each.
trait Base { def x: Int }
inline def make(i: Int): Base = { class Foo extends Base { def x = i }; new Foo }
