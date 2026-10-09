package shared

object CommonUtils:
  def someOrFailText[A](o: Option[A], message: String): Either[String, A] = o.toRight(message)

  extension [A](inst: Set[A])
    def setHolds(elem: A): Boolean = inst.contains(elem)

  extension [A](inst: Option[A])
    def optionHolds(elem: A): Boolean = inst.contains(elem)

  extension[T] (v: T)
    def in(iterable: T*): Boolean =
      iterable.iterator.contains(v)

    def notIn(iterable: T*): Boolean =
      !in(iterable)
