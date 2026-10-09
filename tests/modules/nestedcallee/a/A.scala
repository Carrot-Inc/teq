package nest

// A transparent method of an object nested in an upstream's object, whose expansions the middle
// module's pickles hold: kind 8 names its owner by the path `nest.A$.Inner$`, which the
// downstream's reader resolves through the nested object.
object A:
  object Inner:
    transparent inline def inc(x: Int): Int = x + 1
