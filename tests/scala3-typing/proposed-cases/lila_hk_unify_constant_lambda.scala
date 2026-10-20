// lila: security/EmailAddressValidator.scala:107 (`Domain.Lower.from("duck.com")`, scalalib TotalWrapper)
// Among TotalWrapper's overloaded `from`, `from[M[_]](f: M[Impl])` applies to a plain String:
// M[String] is unified with String.
abstract class TotalWrapper[Newtype, Impl](using Newtype =:= Impl):
  inline def from[M[_]](f: M[Impl]): M[Newtype] = f.asInstanceOf[M[Newtype]]
  final def from[M[_], B](other: TotalWrapper[B, Impl])(f: M[B]): M[Newtype] = f.asInstanceOf[M[Newtype]]

object Domain:
  opaque type Lower = String
  object Lower extends TotalWrapper[Lower, String]

object Main:
  val domain = Domain.Lower.from("duck.com")
  @main def run() = println(domain)
