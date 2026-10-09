package scala.util

/** Evidence that no given instance of `T` is available. The compiler supplies it where a `using`
  * parameter asks for it and the search for `T` finds nothing.
  */
final class NotGiven[+T]

object NotGiven:
  private val cached: NotGiven[Nothing] = new NotGiven[Nothing]
  /** The one instance every search supplies, as scala-library's `value` returns its cached one. */
  def value: NotGiven[Nothing] = cached
