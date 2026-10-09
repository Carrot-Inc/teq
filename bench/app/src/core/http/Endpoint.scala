package meridian.core.http

import meridian.core.json.{JsonCodec, JsonDecoder, JsonEncoder}
import meridian.core.schema.Schema

/** A request part: fixed path text, a captured segment, a query parameter, a header or a body,
  * joined pairwise with the arity rules of [[Concat]]. Each part writes its value into a request
  * and reads it back, consuming path segments in order. */
sealed trait Input[A]:
  def encode(value: A, request: Request): Request
  def decode(request: Request, at: Int): Either[String, (A, Int)]
  def template: String
  def /[B, AB](that: Input[B])(using pc: Concat[A, B, AB]): Input[AB] = Input.Pair(this, that, pc)
  def /(segment: String)(using pc: Concat[A, Unit, A]): Input[A] = Input.Pair(this, Input.Fixed(segment), pc)
  def and[B, AB](that: Input[B])(using pc: Concat[A, B, AB]): Input[AB] = Input.Pair(this, that, pc)
  def map[B](f: A => B)(g: B => A): Input[B] = Input.Mapped(this, g, f)
  def mapDecode[B](f: A => Either[String, B])(g: B => A): Input[B] = Input.MappedDecode(this, f, g)
  def description(text: String): Input[A] = this

object Input:
  final case class Fixed(segment: String) extends Input[Unit]:
    def encode(value: Unit, request: Request) = request.copy(segments = request.segments :+ segment)
    def decode(request: Request, at: Int) =
      if request.segments.lift(at).contains(segment) then Right(((), at + 1)) else Left(s"expected segment $segment")
    def template = segment
  final case class Path[A](name: String, codec: TextCodec[A]) extends Input[A]:
    def encode(value: A, request: Request) = request.copy(segments = request.segments ++ codec.encode(value))
    def decode(request: Request, at: Int) = request.segments.lift(at) match
      case Some(seg) => codec.decode(List(seg)).map(a => (a, at + 1)).left.map(e => s"path $name: $e")
      case None => Left(s"missing segment $name")
    def template = s"{$name}"
  final case class Query[A](name: String, codec: TextCodec[A]) extends Input[A]:
    def encode(value: A, request: Request) = request.copy(query = request.query ++ codec.encode(value).map(v => (name, v)))
    def decode(request: Request, at: Int) = codec.decode(request.query.filter(_._1 == name).map(_._2)).map(a => (a, at)).left.map(e => s"query $name: $e")
    def template = ""
  final case class Header[A](name: String, codec: TextCodec[A]) extends Input[A]:
    def encode(value: A, request: Request) = request.copy(headers = request.headers ++ codec.encode(value).map(v => (name, v)))
    def decode(request: Request, at: Int) = codec.decode(request.headers.filter(_._1 == name).map(_._2)).map(a => (a, at)).left.map(e => s"header $name: $e")
    def template = ""
  final case class Pair[A, B, AB](left: Input[A], right: Input[B], concat: Concat[A, B, AB]) extends Input[AB]:
    def encode(value: AB, request: Request) =
      val (a, b) = concat.split(value)
      right.encode(b, left.encode(a, request))
    def decode(request: Request, at: Int) =
      for
        l <- left.decode(request, at)
        r <- right.decode(request, l._2)
      yield (concat.join(l._1, r._1), r._2)
    def template =
      val (a, b) = (left.template, right.template)
      if a.isEmpty then b else if b.isEmpty then a else s"$a/$b"
  final case class Mapped[A, B](under: Input[A], to: B => A, from: A => B) extends Input[B]:
    def encode(value: B, request: Request) = under.encode(to(value), request)
    def decode(request: Request, at: Int) = under.decode(request, at).map((a, n) => (from(a), n))
    def template = under.template
  final case class MappedDecode[A, B](under: Input[A], to: A => Either[String, B], from: B => A) extends Input[B]:
    def encode(value: B, request: Request) = under.encode(from(value), request)
    def decode(request: Request, at: Int) = under.decode(request, at).flatMap((a, n) => to(a).map(b => (b, n)))
    def template = under.template
  case object Empty extends Input[Unit]:
    def encode(value: Unit, request: Request) = request
    def decode(request: Request, at: Int) = Right(((), at))
    def template = ""

  /** A group of inputs as a case class, the `mapTo` of tapir. */
  extension [A](input: Input[A])
    def mapTo[C](to: A => C)(from: C => A): Input[C] = Mapped(input, from, to)

sealed trait Output[A]:
  def encodeOut(value: A, response: Response): Response
  def decodeOut(response: Response): Either[String, A]
  def and[B, AB](that: Output[B])(using pc: Concat[A, B, AB]): Output[AB] = Output.Pair(this, that, pc)
  def map[B](f: A => B)(g: B => A): Output[B] = Output.Mapped(this, f, g)
  def description(text: String): Output[A] = this

object Output:
  final case class Body[A](codec: JsonCodec[A], schema: Schema[A]) extends Output[A]:
    def encodeOut(value: A, response: Response) = response.copy(body = Some(codec.encodeJson(value)))
    def decodeOut(response: Response) = response.body.toRight("missing body").flatMap(codec.decodeJson)
  final case class Header[A](name: String, codec: TextCodec[A]) extends Output[A]:
    def encodeOut(value: A, response: Response) = response.copy(headers = response.headers ++ codec.encode(value).map(v => (name, v)))
    def decodeOut(response: Response) = codec.decode(response.headers.filter(_._1 == name).map(_._2))
  final case class Status(code: Int) extends Output[Unit]:
    def encodeOut(value: Unit, response: Response) = response.copy(status = code)
    def decodeOut(response: Response) = if response.status == code then Right(()) else Left(s"expected status $code, got ${response.status}")
  final case class Pair[A, B, AB](left: Output[A], right: Output[B], concat: Concat[A, B, AB]) extends Output[AB]:
    def encodeOut(value: AB, response: Response) =
      val (a, b) = concat.split(value)
      right.encodeOut(b, left.encodeOut(a, response))
    def decodeOut(response: Response) =
      for
        l <- left.decodeOut(response)
        r <- right.decodeOut(response)
      yield concat.join(l, r)
  final case class Mapped[A, B](under: Output[A], to: A => B, from: B => A) extends Output[B]:
    def encodeOut(value: B, response: Response) = under.encodeOut(from(value), response)
    def decodeOut(response: Response) = under.decodeOut(response).map(to)
  final case class OneOf[A](variants: List[(Int, Output[A])], pick: A => Int) extends Output[A]:
    def encodeOut(value: A, response: Response) =
      val code = pick(value)
      variants.find(_._1 == code) match
        case Some((_, out)) => out.encodeOut(value, response.copy(status = code))
        case None => response.copy(status = code)
    def decodeOut(response: Response) = variants.find(_._1 == response.status) match
      case Some((_, out)) => out.decodeOut(response)
      case None => Left(s"no variant for status ${response.status}")
  case object Empty extends Output[Unit]:
    def encodeOut(value: Unit, response: Response) = response
    def decodeOut(response: Response) = Right(())

enum Method:
  case Get, Post, Put, Delete

final case class Endpoint[S, I, E, O](method: Method, security: Input[S], input: Input[I], error: Output[E], output: Output[O], tags: List[String]):
  def in[J, IJ](i: Input[J])(using pc: Concat[I, J, IJ]): Endpoint[S, IJ, E, O] = copy(input = Input.Pair(input, i, pc))
  def in(segment: String)(using pc: Concat[I, Unit, I]): Endpoint[S, I, E, O] = copy(input = Input.Pair(input, Input.Fixed(segment), pc))
  def out[P, OP](o: Output[P])(using pc: Concat[O, P, OP]): Endpoint[S, I, E, OP] = copy(output = Output.Pair(output, o, pc))
  def errorOut[F, EF](o: Output[F])(using pc: Concat[E, F, EF]): Endpoint[S, I, EF, O] = copy(error = Output.Pair(error, o, pc))
  def securityIn[T, ST](i: Input[T])(using pc: Concat[S, T, ST]): Endpoint[ST, I, E, O] = copy(security = Input.Pair(security, i, pc))
  def get: Endpoint[S, I, E, O] = copy(method = Method.Get)
  def post: Endpoint[S, I, E, O] = copy(method = Method.Post)
  def put: Endpoint[S, I, E, O] = copy(method = Method.Put)
  def delete: Endpoint[S, I, E, O] = copy(method = Method.Delete)
  def tag(t: String): Endpoint[S, I, E, O] = copy(tags = tags :+ t)
  def summary(text: String): Endpoint[S, I, E, O] = this
  def path: String = input.template

val endpoint: Endpoint[Unit, Unit, Unit, Unit] = Endpoint(Method.Get, Input.Empty, Input.Empty, Output.Empty, Output.Empty, Nil)

def path[A](name: String)(using c: TextCodec[A]): Input[A] = Input.Path(name, c)
def query[A](name: String)(using c: TextCodec[A]): Input[A] = Input.Query(name, c)
def header[A](name: String)(using c: TextCodec[A]): Input[A] = Input.Header(name, c)
def jsonBody[A](using e: JsonEncoder[A], d: JsonDecoder[A], s: Schema[A]): Input[A] & Output[A] = new JsonBody(JsonCodec(e, d), s)
def jsonOut[A](using e: JsonEncoder[A], d: JsonDecoder[A], s: Schema[A]): Output[A] = Output.Body(JsonCodec(e, d), s)
def headerOut[A](name: String)(using c: TextCodec[A]): Output[A] = Output.Header(name, c)
/** The status of the response as an output, or a fixed status with `statusCode(204)`. */
object statusCode extends Output[Int]:
  def apply(code: Int): Output[Unit] = Output.Status(code)
  def encodeOut(value: Int, response: Response) = response.copy(status = value)
  def decodeOut(response: Response) = Right(response.status)
def oneOf[A](variants: (Int, Output[A])*)(pick: A => Int): Output[A] = Output.OneOf(variants.toList, pick)
def oneOfVariant[A](code: Int, out: Output[A]): (Int, Output[A]) = (code, out)

/** A body is an input and an output at once, as tapir's `jsonBody` is. */
final class JsonBody[A](val codec: JsonCodec[A], val schema: Schema[A]) extends Input[A] with Output[A]:
  def encode(value: A, request: Request) = request.copy(body = Some(codec.encodeJson(value)))
  def decode(request: Request, at: Int) = request.body.toRight("missing body").flatMap(codec.decodeJson).map(a => (a, at))
  def template = ""
  def encodeOut(value: A, response: Response) = response.copy(body = Some(codec.encodeJson(value)))
  def decodeOut(response: Response) = response.body.toRight("missing body").flatMap(codec.decodeJson)
  override def map[B](f: A => B)(g: B => A): JsonBody[B] = new JsonBody(codec.transform(f, g), schema.as[B])
  override def description(text: String): JsonBody[A] = this

extension (segment: String)
  def /[B](that: Input[B]): Input[B] = Input.Pair(Input.Fixed(segment), that, Concat.unitLeft[B])
  def /(next: String): Input[Unit] = Input.Pair(Input.Fixed(segment), Input.Fixed(next), Concat.units)
  def toInput: Input[Unit] = Input.Fixed(segment)

final case class Request(method: Method, segments: List[String], query: List[(String, String)], headers: List[(String, String)], body: Option[String]):
  def render: String =
    val q = if query.isEmpty then "" else query.map((k, v) => s"$k=$v").mkString("?", "&", "")
    val h = if headers.isEmpty then "" else headers.map((k, v) => s"$k: $v").mkString(" [", "; ", "]")
    s"$method /${segments.mkString("/")}$q$h${body.map(b => " " + b).getOrElse("")}"

final case class Response(status: Int, headers: List[(String, String)], body: Option[String]):
  def render: String = s"$status${body.map(b => " " + b).getOrElse("")}"

object Wire:
  def encode[A](input: Input[A], value: A, request: Request): Request = input.encode(value, request)
  def decode[A](input: Input[A], request: Request, at: Int): Either[String, (A, Int)] = input.decode(request, at)
  def encodeOut[A](output: Output[A], value: A, response: Response): Response = output.encodeOut(value, response)
  def decodeOut[A](output: Output[A], response: Response): Either[String, A] = output.decodeOut(response)
