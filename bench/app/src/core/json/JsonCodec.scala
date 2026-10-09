package meridian.core.json

import scala.deriving.Mirror

final case class JsonCodec[A](encoder: JsonEncoder[A], decoder: JsonDecoder[A]):
  def transform[B](f: A => B, g: B => A): JsonCodec[B] = JsonCodec(encoder.contramap(g), decoder.map(f))
  def transformOrFail[B](f: A => Either[String, B], g: B => A): JsonCodec[B] = JsonCodec(encoder.contramap(g), decoder.mapOrFail(f))
  def encodeJson(a: A): String = encoder.encodeJson(a)
  def decodeJson(text: String): Either[String, A] = decoder.decodeJson(text)

object JsonCodec:
  def apply[A](using c: JsonCodec[A]): JsonCodec[A] = c
  def from[A](using e: JsonEncoder[A], d: JsonDecoder[A]): JsonCodec[A] = JsonCodec(e, d)
  val string: JsonCodec[String] = JsonCodec(JsonEncoder.string, JsonDecoder.string)
  val int: JsonCodec[Int] = JsonCodec(JsonEncoder.int, JsonDecoder.int)
  val long: JsonCodec[Long] = JsonCodec(JsonEncoder.long, JsonDecoder.long)
  val boolean: JsonCodec[Boolean] = JsonCodec(JsonEncoder.boolean, JsonDecoder.boolean)
  def list[A](using c: JsonCodec[A]): JsonCodec[List[A]] = JsonCodec(JsonEncoder.list(using c.encoder), JsonDecoder.list(using c.decoder))
  def option[A](using c: JsonCodec[A]): JsonCodec[Option[A]] = JsonCodec(JsonEncoder.option(using c.encoder), JsonDecoder.option(using c.decoder))
  inline def derived[A](using m: Mirror.Of[A]): JsonCodec[A] = JsonCodec(JsonEncoder.derived[A](using m), JsonDecoder.derived[A](using m))

object DeriveJsonCodec:
  inline def gen[A](using m: Mirror.Of[A]): JsonCodec[A] = JsonCodec.derived[A](using m)
object DeriveJsonEncoder:
  inline def gen[A](using m: Mirror.Of[A]): JsonEncoder[A] = JsonEncoder.derived[A](using m)
object DeriveJsonDecoder:
  inline def gen[A](using m: Mirror.Of[A]): JsonDecoder[A] = JsonDecoder.derived[A](using m)

/** The case name of a sum goes into the field named here instead of wrapping the object. */
final class jsonDiscriminator(val name: String) extends scala.annotation.StaticAnnotation

extension [A](a: A)
  def toJson(using e: JsonEncoder[A]): String = e.encodeJson(a)
  def toJsonAST(using e: JsonEncoder[A]): Json = e.toJsonAST(a)
extension (text: String)
  def fromJson[A](using d: JsonDecoder[A]): Either[String, A] = d.decodeJson(text)
