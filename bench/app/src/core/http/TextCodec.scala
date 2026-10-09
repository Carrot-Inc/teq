package meridian.core.http

/** A value as a path segment, a query parameter or a header: text in, text out. */
trait TextCodec[A]:
  def encode(a: A): List[String]
  def decode(values: List[String]): Either[String, A]
  def map[B](f: A => B)(g: B => A): TextCodec[B] = TextCodec.instance(b => encode(g(b)), vs => decode(vs).map(f))
  def mapEither[B](f: A => Either[String, B])(g: B => A): TextCodec[B] = TextCodec.instance(b => encode(g(b)), vs => decode(vs).flatMap(f))

object TextCodec:
  def apply[A](using c: TextCodec[A]): TextCodec[A] = c
  def instance[A](enc: A => List[String], dec: List[String] => Either[String, A]): TextCodec[A] = new TextCodec[A]:
    def encode(a: A) = enc(a)
    def decode(values: List[String]) = dec(values)
  def single[A](enc: A => String, dec: String => Either[String, A]): TextCodec[A] = instance(
    a => List(enc(a)),
    {
      case v :: Nil => dec(v)
      case Nil => Left("missing")
      case _ => Left("expected one value")
    })
  given string: TextCodec[String] = single(identity, Right(_))
  given int: TextCodec[Int] = single(_.toString, s => s.toIntOption.toRight(s"invalid Int: $s"))
  given long: TextCodec[Long] = single(_.toString, s => s.toLongOption.toRight(s"invalid Long: $s"))
  given boolean: TextCodec[Boolean] = single(_.toString, {
    case "true" => Right(true)
    case "false" => Right(false)
    case other => Left(s"invalid Boolean: $other")
  })
  given option[A](using c: TextCodec[A]): TextCodec[Option[A]] = instance(
    {
      case Some(a) => c.encode(a)
      case None => Nil
    },
    {
      case Nil => Right(None)
      case vs => c.decode(vs).map(Some(_))
    })
  given list[A](using c: TextCodec[A]): TextCodec[List[A]] = instance(
    as => as.flatMap(c.encode),
    vs => vs.foldRight[Either[String, List[A]]](Right(Nil))((v, acc) => for a <- c.decode(List(v)); rest <- acc yield a :: rest))
