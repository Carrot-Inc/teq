// jars: scala-library scala-java-time scala-java-time-tzdb
// `java.util.TimeZone.toZoneId`, the std's bridge into `java.time`: the JDK's on the JVM,
// scala-java-time's from its jar here, with a region id resolved through the tzdb the program
// registers (`java_time_tzdb.scala`).
import java.time.*
import java.time.zone.{ZoneRulesInitializer, ZoneRulesProvider, TzdbZoneRulesProvider}
import java.util.TimeZone

object Main:
  def main(args: Array[String]): Unit =
    ZoneRulesInitializer.setInitializer(new ZoneRulesInitializer:
      def initializeProviders(): Unit = ZoneRulesProvider.registerProvider(new TzdbZoneRulesProvider()))
    val denver = TimeZone.getTimeZone("America/Denver").toZoneId
    val i = Instant.ofEpochSecond(1709214330L)
    println("" + denver + " " + LocalDateTime.ofInstant(i, denver) + " " + i.atZone(denver).getOffset)
    println(TimeZone.getTimeZone("UTC").toZoneId.getRules.getOffset(i))
    println(i.atZone(TimeZone.getTimeZone("Europe/Paris").toZoneId).toLocalDate)
