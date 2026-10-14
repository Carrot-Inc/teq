package zio

object ZIO:
  def succeed[A](a: A): A = a
class Task
