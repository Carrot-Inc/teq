package meridian.core.schema

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom}

/** The shape of a type as an API document describes it, derived next to the codecs. */
sealed trait SchemaType
object SchemaType:
  case object Text extends SchemaType
  case object Integer extends SchemaType
  case object Number extends SchemaType
  case object Flag extends SchemaType
  case object Unknown extends SchemaType
  final case class Optional(of: SchemaType) extends SchemaType
  final case class Many(of: SchemaType) extends SchemaType
  final case class Keyed(of: SchemaType) extends SchemaType
  final case class Record(name: String, fields: List[(String, SchemaType)]) extends SchemaType
  final case class Choice(name: String, cases: List[(String, SchemaType)]) extends SchemaType
  final case class Named(name: String) extends SchemaType
  final case class Lazy(thunk: () => SchemaType) extends SchemaType

final case class Schema[A](tpe: SchemaType, name: Option[String] = None, description: Option[String] = None):
  def as[B]: Schema[B] = Schema(tpe, name, description)
  def asOption: Schema[Option[A]] = Schema(SchemaType.Optional(tpe), name, description)
  def fieldCount: Int = tpe match
    case SchemaType.Record(_, fields) => fields.length
    case SchemaType.Choice(_, cases) => cases.length
    case _ => 0
  def fieldNames: List[String] = tpe match
    case SchemaType.Record(_, fields) => fields.map(_._1)
    case _ => Nil
  def named(n: String): Schema[A] = copy(name = Some(n))
  def describe(text: String): Schema[A] = copy(description = Some(text))
  def map[B](f: A => B)(g: B => A): Schema[B] = as[B]
  def validate(rule: Validator[A]): Schema[A] = this

final case class Validator[A](check: A => Boolean)
object Validator:
  def enumeration[A](values: List[A]): Validator[A] = Validator(values.contains)
  def positive[A](using n: Numeric[A]): Validator[A] = Validator(a => n.gt(a, n.zero))

object Schema:
  def apply[A](using s: Schema[A]): Schema[A] = s
  def string: Schema[String] = Schema(SchemaType.Text)
  given schemaForString: Schema[String] = string
  given schemaForInt: Schema[Int] = Schema(SchemaType.Integer)
  given schemaForLong: Schema[Long] = Schema(SchemaType.Integer)
  given schemaForBoolean: Schema[Boolean] = Schema(SchemaType.Flag)
  given schemaForUnit: Schema[Unit] = Schema(SchemaType.Unknown)
  given schemaForOption[A](using s: => Schema[A]): Schema[Option[A]] = Schema(SchemaType.Optional(SchemaType.Lazy(() => s.tpe)))
  given schemaForList[A](using s: => Schema[A]): Schema[List[A]] = Schema(SchemaType.Many(SchemaType.Lazy(() => s.tpe)))
  given schemaForSet[A](using s: => Schema[A]): Schema[Set[A]] = Schema(SchemaType.Many(SchemaType.Lazy(() => s.tpe)))
  given schemaForVector[A](using s: => Schema[A]): Schema[Vector[A]] = Schema(SchemaType.Many(SchemaType.Lazy(() => s.tpe)))
  given schemaForMap[K, V](using s: => Schema[V]): Schema[Map[K, V]] = Schema(SchemaType.Keyed(SchemaType.Lazy(() => s.tpe)))
  given schemaForEither[L, R](using l: Schema[L], r: Schema[R]): Schema[Either[L, R]] =
    Schema(SchemaType.Choice("Either", List(("Left", l.tpe), ("Right", r.tpe))))
  given schemaForTuple2[A, B](using a: Schema[A], b: Schema[B]): Schema[(A, B)] =
    Schema(SchemaType.Record("Tuple2", List(("_1", a.tpe), ("_2", b.tpe))))
  def schemaForIterable[A, C[_]](using s: Schema[A]): Schema[C[A]] = Schema(SchemaType.Many(s.tpe))

  inline def derived[A](using m: Mirror.Of[A]): Schema[A] =
    inline m match
      case p: Mirror.ProductOf[A] =>
        val name = constValue[p.MirroredLabel]
        val labels = constValueTuple[p.MirroredElemLabels].toList.asInstanceOf[List[String]]
        val fields = summonAll[Tuple.Map[p.MirroredElemTypes, Schema]].toList.asInstanceOf[List[Schema[Any]]]
        Schema(SchemaType.Record(name, labels.zip(fields.map(_.tpe))))
      case s: Mirror.SumOf[A] =>
        val name = constValue[s.MirroredLabel]
        val labels = constValueTuple[s.MirroredElemLabels].toList.asInstanceOf[List[String]]
        inline if constValue[Tuple.Size[s.MirroredElemTypes]] > 24 then
          Schema(SchemaType.Choice(name, labels.map(l => (l, SchemaType.Record(l, Nil)))))
        else
          Schema(SchemaType.Choice(name, labels.zip(caseSchemas[s.MirroredElemTypes])))

  inline def caseSchemas[T <: Tuple]: List[SchemaType] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => summonOrDerive[a] :: summonOrDerive[b] :: summonOrDerive[c] :: summonOrDerive[d] :: caseSchemas[t]
      case _: (h *: t) => summonOrDerive[h] :: caseSchemas[t]

  inline def summonOrDerive[T]: SchemaType =
    summonFrom {
      case s: Schema[T] => s.tpe
      case m: Mirror.Of[T] => derived[T](using m).tpe
      case _ => SchemaType.Unknown
    }

  def render(tpe: SchemaType): String = render(tpe, 0)

  def render(tpe: SchemaType, depth: Int): String = tpe match
    case SchemaType.Lazy(thunk) => if depth > 4 then "…" else render(thunk(), depth + 1)
    case SchemaType.Text => "text"
    case SchemaType.Integer => "integer"
    case SchemaType.Number => "number"
    case SchemaType.Flag => "flag"
    case SchemaType.Unknown => "?"
    case SchemaType.Optional(of) => render(of, depth) + "?"
    case SchemaType.Many(of) => "[" + render(of, depth) + "]"
    case SchemaType.Keyed(of) => "{" + render(of, depth) + "}"
    case SchemaType.Record(name, fields) => fields.map((f, t) => f + ":" + render(t, depth + 1)).mkString(name + "(", ",", ")")
    case SchemaType.Choice(name, cases) => cases.map((c, t) => c + "=" + render(t, depth + 1)).mkString(name + "<", "|", ">")
    case SchemaType.Named(name) => "@" + name
