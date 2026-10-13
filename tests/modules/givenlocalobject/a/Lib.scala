package gloa

// A given of a local object, imported in the block that defines it, read by a downstream module
// through the method's products.
object L:
  def result: Int =
    object O:
      given n: Int = 42
    import O.given
    summon[Int]
