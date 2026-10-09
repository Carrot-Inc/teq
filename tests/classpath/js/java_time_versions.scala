// jars: scala-java-time
// targets: js interp
// A zone rules provider of the program's own, the shape of the one sbt-tzdb generates into a
// build, whose `provideVersions` answers a `java.util.TreeMap`: scala-java-time registers it and
// hands the map back from `getVersions`, and the program navigates it. The expectation is
// scalac's on the JVM, where the JDK's java.time and TreeMap answer.
import java.time.{Instant, ZoneId, ZoneOffset}
import java.time.zone.{ZoneRules, ZoneRulesProvider}

final class VersionsProvider extends ZoneRulesProvider:
  override protected def provideZoneIds: java.util.Set[String] =
    val zones = new java.util.HashSet[String]()
    zones.add("Test/Versions")
    zones

  override protected def provideRules(regionId: String, forCaching: Boolean): ZoneRules =
    ZoneRules.of(ZoneOffset.ofHours(5))

  override protected def provideVersions(zoneId: String): java.util.NavigableMap[String, ZoneRules] =
    val r = new java.util.TreeMap[String, ZoneRules]
    r.put("2023c", ZoneRules.of(ZoneOffset.ofHours(4)))
    r.put("2024a", ZoneRules.of(ZoneOffset.ofHours(5)))
    r.put("2019b", ZoneRules.of(ZoneOffset.ofHours(3)))
    r

object Main:
  def main(args: Array[String]): Unit =
    ZoneRulesProvider.registerProvider(new VersionsProvider)
    val at = Instant.ofEpochSecond(1709214330L)
    println(ZoneRulesProvider.getRules("Test/Versions", false).getOffset(at))
    println(at.atZone(ZoneId.of("Test/Versions")))
    val versions = ZoneRulesProvider.getVersions("Test/Versions")
    println(s"${versions.size()} ${versions.firstKey()} ${versions.lastKey()} ${versions.keySet()}")
    val last = versions.lastEntry()
    println(s"${last.getKey} ${last.getValue.getOffset(at)}")
    println(s"${versions.floorKey("2020")} ${versions.floorKey("2019b")} ${versions.floorKey("2019")} ${versions.ceilingKey("2024a")} ${versions.ceilingKey("2024b")}")
    println(s"${versions.lowerKey("2019b")} ${versions.higherKey("2023c")} ${versions.higherKey("2024a")}")
    val older = versions.headMap("2024a")
    println(s"${older.keySet()} ${older.size()} ${older.lastKey()}")
    println(s"${versions.descendingKeySet()} ${versions.floorEntry("2023z").getValue.getOffset(at)}")
