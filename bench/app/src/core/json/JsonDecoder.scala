package meridian.core.json

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom}

trait JsonDecoder[A]:
  def fromJsonAST(json: Json): Either[String, A]
  /** What a missing object field decodes to; a failure unless the field may be absent. */
  def unsafeDecodeMissing: Either[String, A] = Left("missing")
  def map[B](f: A => B): JsonDecoder[B] =
    val self = this
    new JsonDecoder[B]:
      def fromJsonAST(json: Json) = self.fromJsonAST(json).map(f)
      override def unsafeDecodeMissing = self.unsafeDecodeMissing.map(f)
  def mapOrFail[B](f: A => Either[String, B]): JsonDecoder[B] =
    val self = this
    new JsonDecoder[B]:
      def fromJsonAST(json: Json) = self.fromJsonAST(json).flatMap(f)
      override def unsafeDecodeMissing = self.unsafeDecodeMissing.flatMap(f)
  def orElse[A1 >: A](that: => JsonDecoder[A1]): JsonDecoder[A1] =
    val self = this
    json => self.fromJsonAST(json) match
      case Left(_) => that.fromJsonAST(json)
      case right => right
  def decodeJson(text: String): Either[String, A] = Json.parse(text).flatMap(fromJsonAST)

object JsonDecoder:
  def apply[A](using d: JsonDecoder[A]): JsonDecoder[A] = d
  def instance[A](f: Json => Either[String, A]): JsonDecoder[A] = json => f(json)

  given string: JsonDecoder[String] =
    case Json.Str(s) => Right(s)
    case _ => Left("expected a string")
  given int: JsonDecoder[Int] =
    case Json.Num(t) => t.toIntOption.toRight(s"expected an Int, got $t")
    case Json.Str(t) => t.toIntOption.toRight(s"expected an Int, got '$t'")
    case _ => Left("expected an Int")
  given long: JsonDecoder[Long] =
    case Json.Num(t) => t.toLongOption.toRight(s"expected a Long, got $t")
    case Json.Str(t) => t.toLongOption.toRight(s"expected a Long, got '$t'")
    case _ => Left("expected a Long")
  given boolean: JsonDecoder[Boolean] =
    case Json.Bool(b) => Right(b)
    case _ => Left("expected a Boolean")
  given char: JsonDecoder[Char] =
    case Json.Str(s) if s.length == 1 => Right(s.charAt(0))
    case _ => Left("expected a Char")
  given unit: JsonDecoder[Unit] = _ => Right(())
  given json: JsonDecoder[Json] = j => Right(j)
  given option[A](using d: JsonDecoder[A]): JsonDecoder[Option[A]] with
    def fromJsonAST(json: Json) = json match
      case Json.Null => Right(None)
      case other => d.fromJsonAST(other).map(Some(_))
    override def unsafeDecodeMissing = Right(None)
  given list[A](using d: JsonDecoder[A]): JsonDecoder[List[A]] =
    case Json.Arr(items) => Derivation.sequence(items.zipWithIndex.map((item, i) => d.fromJsonAST(item).left.map(e => s"[$i]$e")))
    case _ => Left("expected an array")
  given vector[A](using d: JsonDecoder[A]): JsonDecoder[Vector[A]] = list[A].map(_.toVector)
  given set[A](using d: JsonDecoder[A]): JsonDecoder[Set[A]] = list[A].map(_.toSet)
  given map[K, V](using k: JsonFieldDecoder[K], v: JsonDecoder[V]): JsonDecoder[Map[K, V]] =
    case Json.Obj(fields) =>
      Derivation.sequence(fields.map((key, value) =>
        for
          kk <- k.unsafeDecodeField(key).left.map(e => s".$key($e)")
          vv <- v.fromJsonAST(value).left.map(e => s".$key$e")
        yield (kk, vv))).map(_.toMap)
    case _ => Left("expected an object")
  given either[L, R](using l: JsonDecoder[L], r: JsonDecoder[R]): JsonDecoder[Either[L, R]] =
    case Json.Obj(List(("Left", a))) => l.fromJsonAST(a).map(Left(_))
    case Json.Obj(List(("Right", b))) => r.fromJsonAST(b).map(Right(_))
    case _ => Left("expected Left or Right")
  given tuple2[A, B](using a: JsonDecoder[A], b: JsonDecoder[B]): JsonDecoder[(A, B)] =
    case Json.Arr(List(x, y)) => for p <- a.fromJsonAST(x); q <- b.fromJsonAST(y) yield (p, q)
    case _ => Left("expected an array of 2")
  given tuple3[A, B, C](using a: JsonDecoder[A], b: JsonDecoder[B], c: JsonDecoder[C]): JsonDecoder[(A, B, C)] =
    case Json.Arr(List(x, y, z)) => for p <- a.fromJsonAST(x); q <- b.fromJsonAST(y); r <- c.fromJsonAST(z) yield (p, q, r)
    case _ => Left("expected an array of 3")
  given fromCodec[A](using c: => JsonCodec[A]): JsonDecoder[A] = new JsonDecoder[A]:
    def fromJsonAST(json: Json) = c.decoder.fromJsonAST(json)
    override def unsafeDecodeMissing = c.decoder.unsafeDecodeMissing

  inline def derived[A](using m: Mirror.Of[A]): JsonDecoder[A] =
    inline m match
      case p: Mirror.ProductOf[A] =>
        val labels = constValueTuple[p.MirroredElemLabels].toList.asInstanceOf[List[String]]
        val decoders = summonAll[Tuple.Map[p.MirroredElemTypes, JsonDecoder]].toList.asInstanceOf[List[JsonDecoder[Any]]]
        ProductDecoder(labels, decoders, xs => p.fromProduct(Derivation.Elems(xs)))
      case s: Mirror.SumOf[A] =>
        val labels = constValueTuple[s.MirroredElemLabels].toList.asInstanceOf[List[String]]
        inline if constValue[Tuple.Size[s.MirroredElemTypes]] > Derivation.WideSum then
          EnumerationDecoder(labels, JsonAnnotations.enumValuesOf[A])
        else
          val decoders = caseDecoders[s.MirroredElemTypes]
          SumDecoder(labels, decoders, Derivation.allSingletons[s.MirroredElemTypes], JsonAnnotations.discriminatorOf[A])

  inline def caseDecoders[T <: Tuple]: List[JsonDecoder[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) =>
        summonOrDerive[a].asInstanceOf[JsonDecoder[Any]] :: summonOrDerive[b].asInstanceOf[JsonDecoder[Any]] :: summonOrDerive[c].asInstanceOf[JsonDecoder[Any]] :: summonOrDerive[d].asInstanceOf[JsonDecoder[Any]] :: caseDecoders[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[JsonDecoder[Any]] :: caseDecoders[t]

  inline def summonOrDerive[T]: JsonDecoder[T] =
    Deferred(() => summonFrom {
      case d: JsonDecoder[T] => d
      case m: Mirror.Of[T] => derived[T](using m)
    })

  final class Deferred[A](make: () => JsonDecoder[A]) extends JsonDecoder[A]:
    private lazy val under = make()
    def fromJsonAST(json: Json) = under.fromJsonAST(json)
    override def unsafeDecodeMissing = under.unsafeDecodeMissing

  final class ProductDecoder[A](labels: List[String], decoders: List[JsonDecoder[Any]], construct: List[Any] => A) extends JsonDecoder[A]:
    def fromJsonAST(json: Json) = json match
      case Json.Obj(fields) =>
        var values: List[Any] = Nil
        var error: Option[String] = None
        var ls = labels
        var ds = decoders
        while ls.nonEmpty && error.isEmpty do
          val label = ls.head
          val decoded = fields.find(_._1 == label) match
            case Some((_, value)) => ds.head.fromJsonAST(value)
            case None => ds.head.unsafeDecodeMissing
          decoded match
            case Right(v) => values = v :: values
            case Left(e) => error = Some(s".$label($e)")
          ls = ls.tail
          ds = ds.tail
        error.toLeft(construct(values.reverse))
      case Json.Null => Left("expected an object, got null")
      case _ => Left("expected an object")

  /** A sum of singletons decoded by name, for the wide enums whose cases are not walked. */
  final class EnumerationDecoder[A](labels: List[String], values: List[A]) extends JsonDecoder[A]:
    def fromJsonAST(json: Json) = json match
      case Json.Str(name) =>
        labels.indexOf(name) match
          case -1 => Left(s"(invalid enumeration value $name)")
          case i => Right(values(i))
      case _ => Left("(invalid enumeration value)")

  final class SumDecoder[A](labels: List[String], decoders: List[JsonDecoder[Any]], enumeration: Boolean, discriminator: Option[String]) extends JsonDecoder[A]:
    private def decodeCase(name: String, body: Json): Either[String, A] =
      labels.indexOf(name) match
        case -1 => Left(s"(invalid disambiguator $name)")
        case i => decoders(i).fromJsonAST(body).map(_.asInstanceOf[A]).left.map(e => s"{$name}$e")
    def fromJsonAST(json: Json) =
      if enumeration then
        json match
          case Json.Str(name) => decodeCase(name, Json.Obj(Nil))
          case _ => Left("(invalid enumeration value)")
      else
        discriminator match
          case Some(key) =>
            json match
              case obj @ Json.Obj(fields) =>
                fields.find(_._1 == key) match
                  case Some((_, Json.Str(name))) => decodeCase(name, obj)
                  case _ => Left(s"(missing hint '$key')")
              case _ => Left("expected an object")
          case None =>
            json match
              case Json.Obj(List((name, body))) => decodeCase(name, body)
              case Json.Obj(_) => Left("expected a single-key object")
              case _ => Left("expected an object")

trait JsonFieldDecoder[A]:
  def unsafeDecodeField(key: String): Either[String, A]
  def map[B](f: A => B): JsonFieldDecoder[B] = key => unsafeDecodeField(key).map(f)
  def mapOrFail[B](f: A => Either[String, B]): JsonFieldDecoder[B] = key => unsafeDecodeField(key).flatMap(f)
object JsonFieldDecoder:
  def apply[A](using d: JsonFieldDecoder[A]): JsonFieldDecoder[A] = d
  given string: JsonFieldDecoder[String] = key => Right(key)
  given int: JsonFieldDecoder[Int] = key => key.toIntOption.toRight(s"Invalid Int: $key")
  given long: JsonFieldDecoder[Long] = key => key.toLongOption.toRight(s"Invalid Long: $key")

object Derivation:
  /** Sums with more cases than this are enumerations of singletons: their cases are not walked
    * one by one at compile time, since every step is an inline expansion. */
  inline val WideSum = 24

  def sequence[A](xs: List[Either[String, A]]): Either[String, List[A]] =
    xs.foldRight[Either[String, List[A]]](Right(Nil))((x, acc) => for a <- x; rest <- acc yield a :: rest)

  final class Elems(values: List[Any]) extends Product:
    private val array = values.toArray
    def productArity = array.length
    def productElement(n: Int) = array(n)
    def canEqual(that: Any) = false

  inline def allSingletons[T <: Tuple]: Boolean =
    inline erasedValue[T] match
      case _: EmptyTuple => true
      case _: (a *: b *: c *: d *: t) => isSingleton[a] && isSingleton[b] && isSingleton[c] && isSingleton[d] && allSingletons[t]
      case _: (h *: t) => isSingleton[h] && allSingletons[t]

  inline def isSingleton[T]: Boolean =
    summonFrom {
      case p: Mirror.ProductOf[T] => isEmptyProduct[p.MirroredElemTypes]
      case _ => false
    }

  inline def isEmptyProduct[T <: Tuple]: Boolean =
    inline erasedValue[T] match
      case _: EmptyTuple => true
      case _ => false
