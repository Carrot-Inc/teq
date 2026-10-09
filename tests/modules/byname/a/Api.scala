package ba

object Lazy:
  def when[A](cond: Boolean)(body: => A): Option[A] = if cond then Some(body) else None
  def twice(action: => Unit): Unit =
    action
    action
