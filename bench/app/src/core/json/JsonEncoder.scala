package meridian.core.json

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom}

trait JsonEncoder[A]:
  def toJsonAST(a: A): Json
  /** A field whose value has nothing to say is left out of its object. */
  def isNothing(a: A): Boolean = false
  def contramap[B](f: B => A): JsonEncoder[B] =
    val self = this
    new JsonEncoder[B]:
      def toJsonAST(b: B) = self.toJsonAST(f(b))
      override def isNothing(b: B) = self.isNothing(f(b))
  def encodeJson(a: A): String = toJsonAST(a).render

object JsonEncoder:
  def apply[A](using e: JsonEncoder[A]): JsonEncoder[A] = e
  def instance[A](f: A => Json): JsonEncoder[A] = a => f(a)

  given string: JsonEncoder[String] = s => Json.Str(s)
  given int: JsonEncoder[Int] = i => Json.num(i)
  given long: JsonEncoder[Long] = l => Json.num(l)
  given boolean: JsonEncoder[Boolean] = b => Json.Bool(b)
  given char: JsonEncoder[Char] = c => Json.Str(c.toString)
  given unit: JsonEncoder[Unit] = _ => Json.Obj(Nil)
  given json: JsonEncoder[Json] = j => j
  given option[A](using e: JsonEncoder[A]): JsonEncoder[Option[A]] with
    def toJsonAST(a: Option[A]) = a match
      case Some(v) => e.toJsonAST(v)
      case None => Json.Null
    override def isNothing(a: Option[A]) = a.isEmpty
  given list[A](using e: JsonEncoder[A]): JsonEncoder[List[A]] = xs => Json.Arr(xs.map(e.toJsonAST))
  given vector[A](using e: JsonEncoder[A]): JsonEncoder[Vector[A]] = xs => Json.Arr(xs.toList.map(e.toJsonAST))
  given set[A](using e: JsonEncoder[A]): JsonEncoder[Set[A]] = xs => Json.Arr(xs.toList.map(e.toJsonAST))
  given map[K, V](using k: JsonFieldEncoder[K], v: JsonEncoder[V]): JsonEncoder[Map[K, V]] =
    m => Json.Obj(m.toList.collect { case (key, value) if !v.isNothing(value) => (k.unsafeEncodeField(key), v.toJsonAST(value)) })
  given either[L, R](using l: JsonEncoder[L], r: JsonEncoder[R]): JsonEncoder[Either[L, R]] =
    case Left(a) => Json.Obj(List(("Left", l.toJsonAST(a))))
    case Right(b) => Json.Obj(List(("Right", r.toJsonAST(b))))
  given tuple2[A, B](using a: JsonEncoder[A], b: JsonEncoder[B]): JsonEncoder[(A, B)] =
    t => Json.Arr(List(a.toJsonAST(t._1), b.toJsonAST(t._2)))
  given tuple3[A, B, C](using a: JsonEncoder[A], b: JsonEncoder[B], c: JsonEncoder[C]): JsonEncoder[(A, B, C)] =
    t => Json.Arr(List(a.toJsonAST(t._1), b.toJsonAST(t._2), c.toJsonAST(t._3)))
  given fromCodec[A](using c: => JsonCodec[A]): JsonEncoder[A] = new JsonEncoder[A]:
    def toJsonAST(a: A) = c.encoder.toJsonAST(a)
    override def isNothing(a: A) = c.encoder.isNothing(a)

  inline def derived[A](using m: Mirror.Of[A]): JsonEncoder[A] =
    inline m match
      case p: Mirror.ProductOf[A] =>
        val labels = constValueTuple[p.MirroredElemLabels].toList.asInstanceOf[List[String]]
        val encoders = summonAll[Tuple.Map[p.MirroredElemTypes, JsonEncoder]].toList.asInstanceOf[List[JsonEncoder[Any]]]
        ProductEncoder(labels, encoders)
      case s: Mirror.SumOf[A] =>
        val labels = constValueTuple[s.MirroredElemLabels].toList.asInstanceOf[List[String]]
        inline if constValue[Tuple.Size[s.MirroredElemTypes]] > Derivation.WideSum then
          SumEncoder(labels, Nil, a => s.ordinal(a), true, None)
        else
          val encoders = caseEncoders[s.MirroredElemTypes]
          SumEncoder(labels, encoders, a => s.ordinal(a), Derivation.allSingletons[s.MirroredElemTypes], JsonAnnotations.discriminatorOf[A])

  /** The encoders of the cases of a sum: a case with an instance of its own keeps it, the others
    * are derived; four cases per inline step. */
  inline def caseEncoders[T <: Tuple]: List[JsonEncoder[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) =>
        summonOrDerive[a].asInstanceOf[JsonEncoder[Any]] :: summonOrDerive[b].asInstanceOf[JsonEncoder[Any]] :: summonOrDerive[c].asInstanceOf[JsonEncoder[Any]] :: summonOrDerive[d].asInstanceOf[JsonEncoder[Any]] :: caseEncoders[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[JsonEncoder[Any]] :: caseEncoders[t]

  inline def summonOrDerive[T]: JsonEncoder[T] =
    Deferred(() => summonFrom {
      case e: JsonEncoder[T] => e
      case m: Mirror.Of[T] => derived[T](using m)
    })

  final class Deferred[A](make: () => JsonEncoder[A]) extends JsonEncoder[A]:
    private lazy val under = make()
    def toJsonAST(a: A) = under.toJsonAST(a)
    override def isNothing(a: A) = under.isNothing(a)

  final class ProductEncoder[A](labels: List[String], encoders: List[JsonEncoder[Any]]) extends JsonEncoder[A]:
    def toJsonAST(a: A) =
      val values = a.asInstanceOf[Product].productIterator.toList
      var fields: List[(String, Json)] = Nil
      var ls = labels
      var es = encoders
      var vs = values
      while ls.nonEmpty do
        val e = es.head
        if !e.isNothing(vs.head) then fields = (ls.head, e.toJsonAST(vs.head)) :: fields
        ls = ls.tail
        es = es.tail
        vs = vs.tail
      Json.Obj(fields.reverse)

  final class SumEncoder[A](labels: List[String], encoders: List[JsonEncoder[Any]], ordinal: A => Int, enumeration: Boolean, discriminator: Option[String]) extends JsonEncoder[A]:
    def toJsonAST(a: A) =
      val i = ordinal(a)
      if enumeration then Json.Str(labels(i))
      else
        val body = encoders(i).toJsonAST(a)
        discriminator match
          case Some(key) =>
            body match
              case Json.Obj(fields) => Json.Obj((key, Json.Str(labels(i))) :: fields)
              case other => other
          case None => Json.Obj(List((labels(i), body)))

/** The keys of a `Map` field, as zio-json spells them. */
trait JsonFieldEncoder[A]:
  def unsafeEncodeField(a: A): String
  def contramap[B](f: B => A): JsonFieldEncoder[B] = b => unsafeEncodeField(f(b))
object JsonFieldEncoder:
  def apply[A](using e: JsonFieldEncoder[A]): JsonFieldEncoder[A] = e
  given string: JsonFieldEncoder[String] = s => s
  given int: JsonFieldEncoder[Int] = _.toString
  given long: JsonFieldEncoder[Long] = _.toString
