// jars: scala-library scala-java-time scala-java-time-tzdb
// A region zone through the tzdb jar, whose provider scala-java-time's default initializer finds
// by name through portable-scala-reflect: `TzdbZoneRulesProvider` is annotated
// `@EnableReflectiveInstantiation` and nothing names it. The expectation is the JDK's.
import java.time.*

object Main:
  def main(args: Array[String]): Unit =
    val denver = ZoneId.of("America/Denver")
    val i = Instant.ofEpochSecond(1709214330L)
    println("" + denver + " " + denver.getRules.getOffset(i) + " " + denver.getRules.isDaylightSavings(i) + " " + i.atZone(denver))
    val summer = LocalDateTime.of(2024, 7, 4, 12, 0).atZone(denver)
    println("" + summer + " " + summer.toInstant + " " + summer.getOffset + " " + denver.getRules.isDaylightSavings(summer.toInstant))
    val paris = ZoneId.of("Europe/Paris")
    println("" + i.atZone(paris) + " " + i.atZone(paris).withZoneSameInstant(ZoneId.of("Asia/Tokyo")) + " " + ZoneId.of("UTC").getRules.getOffset(i))
    val gap = LocalDateTime.of(2024, 3, 10, 2, 30).atZone(denver)
    println("" + gap + " " + LocalDateTime.of(2024, 11, 3, 1, 30).atZone(denver) + " " + ZonedDateTime.parse("2024-03-10T02:30:00-06:00[America/Denver]"))
