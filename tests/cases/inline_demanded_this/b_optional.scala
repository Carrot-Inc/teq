package demanded

final class Optional[+A >: Null](val value: A):
  def isEmpty = value == null

  inline def getOrElse[B >: A](alt: => B): B =
    if isEmpty then alt else value

  inline def map[B >: Null](f: => A => B): Optional[B] =
    if isEmpty then new Optional(null) else new Optional(f(value))

  override def toString = if isEmpty then "<empty>" else s"$value"

final class Counted(val n: Int):
  def size = n + 1
  inline def twice: Int = size * 2
