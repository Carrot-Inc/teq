package seals

class Seal[T]
given [T]: Seal[T] = Seal[T]()

class Stamps:
  def one[X: Seal] = ()
  def two[X: Seal](x: X) = ()
  def two[X: Seal](x: X, y: X) = ()
  def curried[X: Seal](x: X)(y: X) = ()

class Crate[X: Seal](val x: X)

object Marks:
  extension [X: Seal](x: X) def stamped: Unit = ()

object Olds:
  def old[X](x: X)(implicit s: Seal[X]) = ()
