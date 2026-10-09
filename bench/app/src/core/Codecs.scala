package meridian.core

import meridian.core.amount.Amounts.Amount
import meridian.core.collections.Several
import meridian.core.http.TextCodec
import meridian.core.enums.{Enumerated, Labelled}
import meridian.core.json.*
import meridian.core.keys.{LongKey, TextKey}
import meridian.core.refined.*
import meridian.core.schema.{Schema, SchemaType}
import meridian.core.time.*

/** The instances every id wrapper, every enumeration and the core value types get for free:
  * blanket givens over the type classes the macros derive. */
object Codecs:
  /** An `Ordering` for every `Order`, so that `sorted` and `sortBy` take cats' instances. */
  given orderingFromOrder[A](using o: cats.Order[A]): Ordering[A] = (x, y) => o.compare(x, y)

  given longKeyCodec[K](using key: LongKey[K]): JsonCodec[K] = JsonCodec.long.transform(key.wrap, _.raw)
  given longKeyField[K](using key: LongKey[K]): JsonFieldEncoder[K] = JsonFieldEncoder.long.contramap(_.raw)
  given longKeyFieldDecoder[K](using key: LongKey[K]): JsonFieldDecoder[K] = JsonFieldDecoder.long.map(key.wrap)
  given longKeyEq[K](using key: LongKey[K]): cats.Eq[K] = (a, b) => a.raw == b.raw
  given longKeyOrder[K](using key: LongKey[K]): cats.Order[K] = (a, b) => java.lang.Long.compare(a.raw, b.raw)
  given longKeyShow[K](using key: LongKey[K]): cats.Show[K] = k => k.raw.toString
  given longKeySchema[K](using key: LongKey[K]): Schema[K] = Schema(SchemaType.Integer, Some(key.keyName))
  given longKeyText[K](using key: LongKey[K]): TextCodec[K] = TextCodec.long.mapEither(l => Right(key.wrap(l)))(_.raw)

  given textKeyCodec[K](using key: TextKey[K]): JsonCodec[K] = JsonCodec.string.transform(key.wrap, _.raw)
  given textKeyField[K](using key: TextKey[K]): JsonFieldEncoder[K] = JsonFieldEncoder.string.contramap(_.raw)
  given textKeyFieldDecoder[K](using key: TextKey[K]): JsonFieldDecoder[K] = JsonFieldDecoder.string.map(key.wrap)
  given textKeyEq[K](using key: TextKey[K]): cats.Eq[K] = (a, b) => a.raw == b.raw
  given textKeyOrder[K](using key: TextKey[K]): cats.Order[K] = (a, b) => a.raw.compareTo(b.raw)
  given textKeyShow[K](using key: TextKey[K]): cats.Show[K] = k => k.raw
  given textKeySchema[K](using key: TextKey[K]): Schema[K] = Schema(SchemaType.Text, Some(key.keyName))
  given textKeyText[K](using key: TextKey[K]): TextCodec[K] = TextCodec.string.mapEither(key.parse)(_.raw)

  given enumeratedEncoder[E](using e: Enumerated[E]): JsonEncoder[E] = JsonEncoder.string.contramap(_.entryName)
  given enumeratedDecoder[E](using e: Enumerated[E]): JsonDecoder[E] =
    JsonDecoder.string.mapOrFail(s => e.valueOfIgnoreCase(s).toRight(s"Unsupported ${e.enumName}: $s"))
  given enumeratedCodec[E](using e: Enumerated[E]): JsonCodec[E] = JsonCodec(enumeratedEncoder, enumeratedDecoder)
  given enumeratedField[E](using e: Enumerated[E]): JsonFieldEncoder[E] = JsonFieldEncoder.string.contramap(_.entryName)
  given enumeratedFieldDecoder[E](using e: Enumerated[E]): JsonFieldDecoder[E] =
    JsonFieldDecoder.string.mapOrFail(s => e.valueOfIgnoreCase(s).toRight(s"Unsupported ${e.enumName}: $s"))
  given enumeratedEq[E](using e: Enumerated[E]): cats.Eq[E] = (a, b) => a == b
  given enumeratedOrder[E](using e: Enumerated[E]): cats.Order[E] = (a, b) => Integer.compare(e.ordinalOf(a), e.ordinalOf(b))
  given enumeratedShow[E](using e: Enumerated[E]): cats.Show[E] = _.entryName
  given enumeratedSchema[E](using e: Enumerated[E]): Schema[E] = Schema(SchemaType.Text, Some(e.enumName))
  given enumeratedText[E](using e: Enumerated[E]): TextCodec[E] =
    TextCodec.string.mapEither(s => e.valueOfIgnoreCase(s).toRight(s"Unsupported ${e.enumName}: $s"))(_.entryName)

  given labelledShow[E](using l: Labelled[E]): cats.Show[E] = _.entryName

  given instantCodec: JsonCodec[Instant] = JsonCodec.string.transformOrFail(s => Instant.parseOption(s).toRight("expected an Instant"), _.toString)
  given instantSchema: Schema[Instant] = Schema(SchemaType.Text, Some("Instant"))
  given instantEq: cats.Eq[Instant] = (a, b) => a == b
  given instantOrder: cats.Order[Instant] = (a, b) => a.compare(b)
  given instantShow: cats.Show[Instant] = _.toString
  given instantText: TextCodec[Instant] = TextCodec.string.mapEither(s => Instant.parseOption(s).toRight("expected an Instant"))(_.toString)

  given localDateCodec: JsonCodec[LocalDate] = JsonCodec.string.transformOrFail(s => LocalDate.parseOption(s).toRight("expected a LocalDate"), _.toString)
  given localDateField: JsonFieldEncoder[LocalDate] = JsonFieldEncoder.string.contramap(_.toString)
  given localDateDecoder: JsonFieldDecoder[LocalDate] = JsonFieldDecoder.string.mapOrFail(s => LocalDate.parseOption(s).toRight("expected a LocalDate"))
  given localDateSchema: Schema[LocalDate] = Schema(SchemaType.Text, Some("LocalDate"))
  given localDateEq: cats.Eq[LocalDate] = (a, b) => a == b
  given localDateOrdering: cats.Order[LocalDate] = (a, b) => a.compare(b)
  given localDateShow: cats.Show[LocalDate] = _.toString
  given localDateText: TextCodec[LocalDate] = TextCodec.string.mapEither(s => LocalDate.parseOption(s).toRight("expected a LocalDate"))(_.toString)

  given localTimeCodec: JsonCodec[LocalTime] = JsonCodec.string.transformOrFail(s => LocalTime.parseOption(s).toRight("expected a LocalTime"), _.toString)
  given localTimeSchema: Schema[LocalTime] = Schema(SchemaType.Text, Some("LocalTime"))
  given localTimeEq: cats.Eq[LocalTime] = (a, b) => a == b
  given localTimeOrder: cats.Order[LocalTime] = (a, b) => a.compare(b)
  given localTimeShow: cats.Show[LocalTime] = _.toString

  given durationCodec: JsonCodec[Duration] = JsonCodec.long.transform(Duration.ofMillis, _.toMillis)
  given durationSchema: Schema[Duration] = Schema(SchemaType.Integer, Some("Duration"))
  given durationEq: cats.Eq[Duration] = (a, b) => a == b
  given durationShow: cats.Show[Duration] = _.toString

  given dayOfWeekCodec: JsonCodec[DayOfWeek] = JsonCodec.string.transformOrFail(s => DayOfWeek.values.find(_.toString == s).toRight(s"Unsupported DayOfWeek: $s"), _.toString)
  given dayOfWeekSchema: Schema[DayOfWeek] = Schema(SchemaType.Text, Some("DayOfWeek"))
  given dayOfWeekEq: cats.Eq[DayOfWeek] = (a, b) => a == b

  given amountCodec: JsonCodec[Amount] = JsonCodec.string.transformOrFail(Amount.parse, _.format(4))
  given amountSchema: Schema[Amount] = Schema(SchemaType.Number, Some("Amount"))
  given amountEq: cats.Eq[Amount] = (a, b) => a == b

  given posIntCodec: JsonCodec[PosInt] = JsonCodec.int.transformOrFail(PosInt.from, _.value)
  given posIntSchema: Schema[PosInt] = Schema(SchemaType.Integer, Some("PosInt"))
  given posIntEq: cats.Eq[PosInt] = (a, b) => a.value == b.value
  given nonNegIntCodec: JsonCodec[NonNegInt] = JsonCodec.int.transformOrFail(NonNegInt.from, _.value)
  given nonNegIntSchema: Schema[NonNegInt] = Schema(SchemaType.Integer, Some("NonNegInt"))
  given nonNegIntEq: cats.Eq[NonNegInt] = (a, b) => a.value == b.value
  given nonEmptyTextCodec: JsonCodec[NonEmptyText] = JsonCodec.string.transformOrFail(NonEmptyText.from, _.value)
  given nonEmptyTextSchema: Schema[NonEmptyText] = Schema(SchemaType.Text, Some("NonEmptyText"))
  given nonEmptyTextEq: cats.Eq[NonEmptyText] = (a, b) => a.value == b.value
  given percentCodec: JsonCodec[Percent] = JsonCodec.int.transformOrFail(Percent.from, _.value)
  given percentSchema: Schema[Percent] = Schema(SchemaType.Integer, Some("Percent"))
  given percentEq: cats.Eq[Percent] = (a, b) => a.value == b.value

  given severalEncoder[A](using e: JsonEncoder[A]): JsonEncoder[Several[A]] = JsonEncoder.list(using e).contramap(_.toList)
  given severalDecoder[A](using d: JsonDecoder[A]): JsonDecoder[Several[A]] =
    JsonDecoder.list(using d).mapOrFail(xs => Several.fromList(xs).toRight("Several cannot be empty"))
  given severalCodec[A](using e: JsonEncoder[A], d: JsonDecoder[A]): JsonCodec[Several[A]] = JsonCodec(severalEncoder, severalDecoder)
  given severalSchema[A](using s: Schema[A]): Schema[Several[A]] = Schema(SchemaType.Many(s.tpe))
  given severalEq[A](using e: cats.Eq[A]): cats.Eq[Several[A]] = (a, b) => a.length == b.length && a.toList.zip(b.toList).forall((x, y) => e.eqv(x, y))
  given severalShow[A](using s: cats.Show[A]): cats.Show[Several[A]] = xs => xs.toList.map(s.show).mkString("Several(", ", ", ")")

  given jsonSchema: Schema[Json] = Schema(SchemaType.Unknown, Some("Json"))
  given jsonCodec: JsonCodec[Json] = JsonCodec(JsonEncoder.json, JsonDecoder.json)
