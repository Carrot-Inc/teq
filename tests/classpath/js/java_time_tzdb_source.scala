// jars: scala-java-time
// targets: js interp
// The zone rules provider from source, as sbt-tzdb generates it into a build: scala-java-time's
// default initializer finds `java.time.zone.TzdbZoneRulesProvider` by name, the program's own
// class this time. The expectation is Scala.js's.
package java.time.zone:

  import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

  @EnableReflectiveInstantiation
  final class TzdbZoneRulesProvider extends ZoneRulesProvider:
    override protected def provideZoneIds: java.util.Set[String] =
      val zones = new java.util.HashSet[String]()
      zones.add("Test/Plus5")
      zones

    override protected def provideRules(regionId: String, forCaching: Boolean): ZoneRules =
      ZoneRules.of(java.time.ZoneOffset.ofHours(5))

    override protected def provideVersions(zoneId: String): java.util.NavigableMap[String, ZoneRules] =
      new java.util.TreeMap[String, ZoneRules]()

package app:

  import java.time.*

  object Main:
    def main(args: Array[String]): Unit =
      val zone = ZoneId.of("Test/Plus5")
      val i = Instant.ofEpochSecond(1709214330L)
      println(s"$zone ${zone.getRules.getOffset(i)} ${i.atZone(zone)}")
      println(ZoneId.getAvailableZoneIds.contains("Test/Plus5"))
