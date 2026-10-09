// jars: scala-library kantan-csv kantan-codecs kantan-csv-java8 kantan-codecs-java8
// std: scala-library
//> using dep com.nrinaudo:kantan.csv-java8_2.13:0.8.0
// kantan.csv (a Scala 2.13 jar) writing rows with its java8 module's `Instant` and `LocalDate`
// cells in link mode: `RowEncoder.caseOrdered` over a case class, `CellEncoder.from` and a
// contramapped encoder, then `asCsv` with a header.
import kantan.csv.*
import kantan.csv.ops.*
import kantan.csv.java8.*
import java.time.{Instant, LocalDate}

enum Kind:
  case Retail, Wholesale

final case class Email(value: String)
final case class Row(id: Long, kind: Kind, email: Email, day: LocalDate, seen: Option[Instant], made: Instant)

object Row:
  implicit val kindCell: CellEncoder[Kind] = CellEncoder.from {
    case Kind.Retail => "retail"
    case Kind.Wholesale => "wholesale"
  }
  implicit val emailCell: CellEncoder[Email] = implicitly[CellEncoder[String]].contramap(_.value)
  implicit val rowEncoder: RowEncoder[Row] =
    RowEncoder.caseOrdered[Row, Long, Kind, Email, LocalDate, Option[Instant], Instant](r => Some((r.id, r.kind, r.email, r.day, r.seen, r.made)))

object Main:
  def main(args: Array[String]): Unit =
    val rows = List(
      Row(1L, Kind.Retail, Email("a@b.c"), LocalDate.of(2026, 9, 28), Some(Instant.EPOCH), Instant.parse("2026-01-02T03:04:05Z")),
      Row(2L, Kind.Wholesale, Email("d@e.f"), LocalDate.of(2000, 1, 1), None, Instant.EPOCH),
    )
    print(rows.asCsv(rfc.withHeader("id", "kind", "email", "day", "seen", "made")))
