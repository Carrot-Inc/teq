package pairs

object Pairs:
  extension [A](x: A)
    inline def pair[B](y: B): (A, B) = (x, y)
