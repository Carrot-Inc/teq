// jars: scala-library zio-json magnolia zio scala-java-time
//> using dep dev.zio::zio-json:0.9.2
// zio-json's java.time codecs through the jar path: a case class over Instant, LocalDate,
// LocalDateTime, LocalTime, ZoneOffset and Duration, derived, encoded and decoded.
import zio.json.*
import java.time.*

case class Event(at: Instant, day: LocalDate, when: LocalDateTime, time: LocalTime, offset: ZoneOffset, took: Duration) derives JsonCodec

object Main:
  def main(args: Array[String]): Unit =
    val e = Event(Instant.ofEpochMilli(1709214330500L), LocalDate.of(2024, 2, 29), LocalDateTime.of(2024, 2, 29, 13, 45, 30), LocalTime.of(13, 45, 30, 123000000), ZoneOffset.ofHours(2), Duration.ofSeconds(3725, 500000000L))
    val json = e.toJson
    println(json)
    println(json.fromJson[Event])
    println(json.fromJson[Event] == Right(e))
    println("""{"at":"2024-02-29T13:45:30Z","day":"2024-03-01","when":"2024-02-29T00:00:00","time":"08:30","offset":"Z","took":"PT1H"}""".fromJson[Event])
    println("""{"at":"not a time","day":"2024-03-01","when":"2024-02-29T00:00:00","time":"08:30","offset":"Z","took":"PT1H"}""".fromJson[Event])
    println("""{"at":"2024-02-29T13:45:30Z","day":"2024-02-30","when":"2024-02-29T00:00:00","time":"08:30","offset":"Z","took":"PT1H"}""".fromJson[Event])
    println(List(LocalDate.of(2024, 1, 1), LocalDate.of(2024, 12, 31)).toJson + " " + "[\"2024-01-01\"]".fromJson[List[LocalDate]] + " " + Duration.ofMillis(1500).toJson + " " + "\"PT0.5S\"".fromJson[Duration])
