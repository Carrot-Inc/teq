package lib

// Aliases of an object: the forwarder reads them as the object's type members.
object syntax:
  type Same[A] = lib.kernel.Eq[A]
  type SamePair[A] = lib.kernel.Eq[(A, A)]
