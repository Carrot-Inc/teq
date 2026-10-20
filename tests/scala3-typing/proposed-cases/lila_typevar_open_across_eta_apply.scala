// lila: app/controllers/Ublog.scala:262 (`limit.imageUpload(rateLimited): env.ublog.api.image.upload(...)`)
// `imageUpload[A](limited)` returns the eta-expanded `(=> Option[A]) => Option[A]`; its A stays
// open while that function is applied to the block, so A becomes String | Int, not String.
class Limiter:
  def apply[A](k: String, default: => A)(op: => A): A = op

object limit:
  private val lim = Limiter()
  def imageUpload[A](limited: => Option[A]) = lim("k", limited)

@main def run() =
  val r = limit.imageUpload(Option("limited")):
    Option(42)
  println(r)
