package xforum

// An inline member declared without a result forwards the one scalac infers from its body, as dotty's
// forwarder takes the member's completed info: `not` re-exported by a package (lila's
// `Fu[Boolean].not` through `export lila.core.lilaism.Lilaism.*`), its result what the inference of
// `filterA`'s `G` reads (before, the forwarder's result was in error: no `Applicative[Nothing]`).
import xcore.Wrap.Box

trait Applicative[F[_]]
object Applicative:
  given Applicative[Box] with {}

extension [A](xs: List[A])
  def filterA[G[_]](f: A => G[Boolean])(using Applicative[G]): List[G[Boolean]] = xs.map(f)

@main def run(): Unit =
  def blocks(u: String): Box[Boolean] = Box(u == "b")
  println(List("a", "b").filterA(blocks(_).not))
  val n: Box[Boolean] = blocks("a").not
  println(n)
