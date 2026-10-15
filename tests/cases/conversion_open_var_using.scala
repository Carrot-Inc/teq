import scala.language.implicitConversions

// A conversion's variable left open for the member it serves, named by the member's using clause
// (zio-json's `EncoderOps[A](xs).toJson(using JsonEncoder[A])` over an invariant encoder): it takes
// the receiver's type before the search, as dotty's `adaptNoArgsImplicitMethod` instantiates the
// prefix's variables its implicit parameters name (`Inferencing.tvarsInParams`), so `list` and not
// `seq` answers; one the using clause does not name stays open (cats' `pure[F]`).
trait Enc[A]:
  def name: String
trait EncLow:
  implicit def seq[A](implicit a: Enc[A]): Enc[Seq[A]] = new Enc[Seq[A]] { def name = "seq of " + a.name }
  implicit def list[A](implicit a: Enc[A]): Enc[List[A]] = new Enc[List[A]] { def name = "list of " + a.name }
object Enc extends EncLow:
  implicit val double: Enc[Double] = new Enc[Double] { def name = "double" }

final class EncoderOps[A](private val a: A) extends AnyVal:
  def encoded(implicit e: Enc[A]): String = e.name
implicit def encoderOps[A](a: A): EncoderOps[A] = new EncoderOps(a)

@main def run(): Unit =
  println(List(0.5, 1.5).encoded)
  println(Seq(2.5).encoded)
