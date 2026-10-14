package xcore

object Wrap:
  final case class Box[A](a: A):
    def map[B](f: A => B): Box[B] = Box(f(a))

trait LibExtensions:
  extension (b: Wrap.Box[Boolean])
    inline def not = b.map(!_)

object Lilaism extends LibExtensions
