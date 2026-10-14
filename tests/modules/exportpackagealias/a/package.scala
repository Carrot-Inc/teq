package object lib:
  type Eq[A] = lib.kernel.Eq[A]
  // An alias that is no renaming: the forwarder keeps it.
  type EqPair[A] = lib.kernel.Eq[(A, A)]
